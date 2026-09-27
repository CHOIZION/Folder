#[cfg(target_os = "windows")]
mod windows_encoder {
    use std::{ffi::c_void, mem::ManuallyDrop, ptr, slice};

    use windows::{
        core::{Interface, Result as WindowsResult},
        Win32::{
            Media::MediaFoundation::*,
            System::{
                Com::{
                    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
                },
                Variant::VARIANT,
            },
        },
    };

    const INPUT_STREAM_ID: u32 = 0;
    const OUTPUT_STREAM_ID: u32 = 0;
    const HUNDRED_NS_PER_SECOND: i64 = 10_000_000;

    #[derive(Clone, Copy)]
    struct EncoderConfig {
        width: u32,
        height: u32,
        fps: u32,
        bitrate: u32,
    }

    #[derive(Debug, Clone)]
    pub struct EncodedAccessUnit {
        pub data: Vec<u8>,
        pub timestamp_us: u64,
        pub keyframe: bool,
        pub has_configuration: bool,
    }

    pub struct H264Encoder {
        transform: IMFTransform,
        activation: Option<IMFActivate>,
        event_generator: Option<IMFMediaEventGenerator>,
        codec_api: Option<ICodecAPI>,
        output_buffer_size: u32,
        width: u32,
        height: u32,
        input_requests: u32,
        sequence_header: Vec<u8>,
        encoder_name: String,
        hardware: bool,
        frame_duration_100ns: i64,
        trace_events: bool,
        _session: MediaFoundationSession,
    }

    impl H264Encoder {
        pub fn new(width: u32, height: u32, fps: u32, bitrate: u32) -> Result<Self, String> {
            if width == 0
                || height == 0
                || !width.is_multiple_of(2)
                || !height.is_multiple_of(2)
                || fps == 0
            {
                return Err("H.264 화면 크기는 양수인 짝수여야 합니다.".to_string());
            }
            let config = EncoderConfig {
                width,
                height,
                fps,
                bitrate,
            };

            let mut session = MediaFoundationSession::new()?;
            let mut hardware_errors = Vec::new();
            for (activation, name) in enumerate_hardware_encoders()? {
                let transform = match unsafe { activation.ActivateObject::<IMFTransform>() } {
                    Ok(transform) => transform,
                    Err(error) => {
                        hardware_errors.push(format!("{name}: 활성화 실패 ({error})"));
                        continue;
                    }
                };
                match Self::configure(
                    transform,
                    Some(activation.clone()),
                    name.clone(),
                    true,
                    config,
                    session,
                ) {
                    Ok(encoder) => return Ok(encoder),
                    Err((error, recovered_session)) => {
                        hardware_errors.push(format!("{name}: {error}"));
                        session = recovered_session;
                    }
                }
            }

            Self::software_or_error(config, session, hardware_errors)
        }

        fn software_or_error(
            config: EncoderConfig,
            session: MediaFoundationSession,
            hardware_errors: Vec<String>,
        ) -> Result<Self, String> {
            let transform: IMFTransform = unsafe {
                CoCreateInstance(&CMSH264EncoderMFT, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| format!("Microsoft H.264 인코더를 열지 못했습니다: {error}"))?
            };
            match Self::configure(
                transform,
                None,
                "Microsoft H.264 Encoder MFT (software fallback)".to_string(),
                false,
                EncoderConfig {
                    bitrate: config.bitrate.min(10_000_000),
                    ..config
                },
                session,
            ) {
                Ok(encoder) => Ok(encoder),
                Err((software_error, _)) => {
                    let hardware_detail = if hardware_errors.is_empty() {
                        "사용 가능한 하드웨어 H.264 인코더가 없습니다.".to_string()
                    } else {
                        hardware_errors.join("; ")
                    };
                    Err(format!(
                        "H.264 인코더 초기화 실패. 하드웨어: {hardware_detail} 소프트웨어: {software_error}"
                    ))
                }
            }
        }

        fn configure(
            transform: IMFTransform,
            activation: Option<IMFActivate>,
            encoder_name: String,
            hardware: bool,
            config: EncoderConfig,
            session: MediaFoundationSession,
        ) -> Result<Self, (String, MediaFoundationSession)> {
            let configured = unsafe {
                configure_transform(
                    &transform,
                    config.width,
                    config.height,
                    config.fps,
                    config.bitrate,
                )
                .and_then(|_| transform.GetOutputStreamInfo(OUTPUT_STREAM_ID))
            };
            let output_info = match configured {
                Ok(info) => info,
                Err(error) => {
                    return Err((format!("미디어 형식 설정 실패 ({error})"), session));
                }
            };

            let attributes = match unsafe { transform.GetAttributes() } {
                Ok(attributes) => attributes,
                Err(error) => {
                    return Err((format!("인코더 속성 조회 실패 ({error})"), session));
                }
            };
            let asynchronous =
                unsafe { attributes.GetUINT32(&MF_TRANSFORM_ASYNC) }.unwrap_or(0) != 0;
            let event_generator = if asynchronous {
                if let Err(error) = unsafe { attributes.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1) } {
                    return Err((format!("비동기 인코더 잠금 해제 실패 ({error})"), session));
                }
                match transform.cast::<IMFMediaEventGenerator>() {
                    Ok(generator) => Some(generator),
                    Err(error) => {
                        return Err((format!("인코더 이벤트 연결 실패 ({error})"), session));
                    }
                }
            } else {
                None
            };

            let codec_api = transform.cast::<ICodecAPI>().ok();
            if let Some(api) = &codec_api {
                set_codec_value(api, &CODECAPI_AVEncCommonLowLatency, VARIANT::from(true));
                set_codec_value(api, &CODECAPI_AVLowLatencyMode, VARIANT::from(true));
                set_codec_value(
                    api,
                    &CODECAPI_AVEncMPVDefaultBPictureCount,
                    VARIANT::from(0_u32),
                );
                set_codec_value(api, &CODECAPI_AVEncMPVGOPSize, VARIANT::from(config.fps));
                set_codec_value(
                    api,
                    &CODECAPI_AVEncCommonRateControlMode,
                    VARIANT::from(eAVEncCommonRateControlMode_LowDelayVBR.0 as u32),
                );
                set_codec_value(
                    api,
                    &CODECAPI_AVEncCommonMeanBitRate,
                    VARIANT::from(config.bitrate),
                );
            }

            if let Err(error) = unsafe {
                transform
                    .ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0)
                    .and_then(|_| transform.ProcessMessage(MFT_MESSAGE_NOTIFY_BEGIN_STREAMING, 0))
                    .and_then(|_| transform.ProcessMessage(MFT_MESSAGE_NOTIFY_START_OF_STREAM, 0))
            } {
                return Err((format!("인코더 스트림 시작 실패 ({error})"), session));
            }

            let sequence_header = output_sequence_header(&transform).unwrap_or_default();
            Ok(Self {
                transform,
                activation,
                event_generator,
                codec_api,
                output_buffer_size: output_info
                    .cbSize
                    .max(config.width.saturating_mul(config.height))
                    .max(1_048_576),
                width: config.width,
                height: config.height,
                input_requests: 0,
                sequence_header,
                encoder_name,
                hardware,
                frame_duration_100ns: HUNDRED_NS_PER_SECOND / i64::from(config.fps),
                trace_events: std::env::var_os("HEART_TEST_H264").is_some(),
                _session: session,
            })
        }

        pub fn encoder_name(&self) -> &str {
            &self.encoder_name
        }

        pub fn is_hardware(&self) -> bool {
            self.hardware
        }

        pub fn force_keyframe(&self) {
            if let Some(api) = &self.codec_api {
                set_codec_value(api, &CODECAPI_AVEncVideoForceKeyFrame, VARIANT::from(true));
            }
        }

        pub fn encode(
            &mut self,
            nv12: &[u8],
            timestamp_100ns: i64,
        ) -> Result<Vec<EncodedAccessUnit>, String> {
            let expected = self.width as usize * self.height as usize * 3 / 2;
            if nv12.len() != expected {
                return Err(format!(
                    "NV12 프레임 크기가 올바르지 않습니다: {} != {expected}",
                    nv12.len()
                ));
            }

            let mut output = Vec::with_capacity(2);
            if self.event_generator.is_some() {
                self.wait_for_input_request(&mut output)?;
            }

            let sample = create_input_sample(nv12, timestamp_100ns, self.frame_duration_100ns)
                .map_err(|error| format!("H.264 입력 프레임 생성 실패: {error}"))?;
            match unsafe { self.transform.ProcessInput(INPUT_STREAM_ID, &sample, 0) } {
                Ok(()) => {
                    self.input_requests = self.input_requests.saturating_sub(1);
                }
                Err(error) if error.code() == MF_E_NOTACCEPTING => {
                    self.drain_output(&mut output)?;
                    unsafe { self.transform.ProcessInput(INPUT_STREAM_ID, &sample, 0) }
                        .map_err(|retry| format!("H.264 입력 거부: {error}; 재시도: {retry}"))?;
                    self.input_requests = self.input_requests.saturating_sub(1);
                }
                Err(error) => return Err(format!("H.264 입력 실패: {error}")),
            }

            if self.event_generator.is_some() {
                self.drain_async_events(&mut output)?;
            } else {
                self.drain_output(&mut output)?;
            }
            Ok(output)
        }

        pub fn poll(&mut self) -> Result<Vec<EncodedAccessUnit>, String> {
            let mut output = Vec::with_capacity(2);
            if self.event_generator.is_some() {
                self.drain_async_events(&mut output)?;
            } else {
                self.drain_output(&mut output)?;
            }
            Ok(output)
        }

        fn wait_for_input_request(
            &mut self,
            output: &mut Vec<EncodedAccessUnit>,
        ) -> Result<(), String> {
            while self.input_requests == 0 {
                let generator = self
                    .event_generator
                    .as_ref()
                    .ok_or_else(|| "비동기 H.264 이벤트 생성기가 없습니다.".to_string())?;
                let event = unsafe { generator.GetEvent(MEDIA_EVENT_GENERATOR_GET_EVENT_FLAGS(0)) }
                    .map_err(|error| format!("H.264 입력 이벤트 대기 실패: {error}"))?;
                self.handle_event(&event, output)?;
            }
            Ok(())
        }

        fn drain_async_events(
            &mut self,
            output: &mut Vec<EncodedAccessUnit>,
        ) -> Result<(), String> {
            loop {
                let generator = self
                    .event_generator
                    .as_ref()
                    .ok_or_else(|| "비동기 H.264 이벤트 생성기가 없습니다.".to_string())?;
                match unsafe { generator.GetEvent(MF_EVENT_FLAG_NO_WAIT) } {
                    Ok(event) => self.handle_event(&event, output)?,
                    Err(error) if error.code() == MF_E_NO_EVENTS_AVAILABLE => return Ok(()),
                    Err(error) => return Err(format!("H.264 이벤트 조회 실패: {error}")),
                }
            }
        }

        fn handle_event(
            &mut self,
            event: &IMFMediaEvent,
            output: &mut Vec<EncodedAccessUnit>,
        ) -> Result<(), String> {
            let status = unsafe { event.GetStatus() }
                .map_err(|error| format!("H.264 이벤트 상태 조회 실패: {error}"))?;
            status
                .ok()
                .map_err(|error| format!("H.264 인코더 이벤트 오류: {error}"))?;
            let event_type = unsafe { event.GetType() }
                .map_err(|error| format!("H.264 이벤트 종류 조회 실패: {error}"))?;
            if self.trace_events {
                eprintln!("HEART H.264 event: {event_type}");
            }
            if event_type == METransformNeedInput.0 as u32 {
                self.input_requests = self.input_requests.saturating_add(1);
            } else if event_type == METransformHaveOutput.0 as u32 {
                if let Some(unit) = self.process_output_once()? {
                    output.push(unit);
                }
            }
            Ok(())
        }

        fn drain_output(&mut self, output: &mut Vec<EncodedAccessUnit>) -> Result<(), String> {
            loop {
                match self.process_output_once() {
                    Ok(Some(unit)) => output.push(unit),
                    Ok(None) => return Ok(()),
                    Err(error) => return Err(error),
                }
            }
        }

        fn process_output_once(&mut self) -> Result<Option<EncodedAccessUnit>, String> {
            let info = unsafe { self.transform.GetOutputStreamInfo(OUTPUT_STREAM_ID) }
                .map_err(|error| format!("H.264 출력 정보 조회 실패: {error}"))?;
            let provides_sample = info.dwFlags
                & (MFT_OUTPUT_STREAM_PROVIDES_SAMPLES.0 as u32
                    | MFT_OUTPUT_STREAM_CAN_PROVIDE_SAMPLES.0 as u32)
                != 0;
            if self.trace_events {
                eprintln!(
                    "HEART H.264 ProcessOutput flags=0x{:x} cbSize={} provides={}",
                    info.dwFlags, info.cbSize, provides_sample
                );
            }
            let supplied_sample = if provides_sample {
                None
            } else {
                Some(
                    create_output_sample(self.output_buffer_size)
                        .map_err(|error| format!("H.264 출력 버퍼 생성 실패: {error}"))?,
                )
            };
            let mut output_buffer = MFT_OUTPUT_DATA_BUFFER {
                dwStreamID: OUTPUT_STREAM_ID,
                pSample: ManuallyDrop::new(supplied_sample),
                dwStatus: 0,
                pEvents: ManuallyDrop::new(None),
            };
            let mut status = 0_u32;
            let result = unsafe {
                self.transform
                    .ProcessOutput(0, slice::from_mut(&mut output_buffer), &mut status)
            };
            let sample = unsafe { ManuallyDrop::take(&mut output_buffer.pSample) };
            let events = unsafe { ManuallyDrop::take(&mut output_buffer.pEvents) };
            drop(events);

            if let Err(error) = result {
                if error.code() == MF_E_TRANSFORM_NEED_MORE_INPUT {
                    return Ok(None);
                }
                if error.code() == MF_E_TRANSFORM_STREAM_CHANGE {
                    self.sequence_header =
                        output_sequence_header(&self.transform).unwrap_or_default();
                    return Ok(None);
                }
                return Err(format!("H.264 출력 실패: {error}"));
            }
            let Some(sample) = sample else {
                return Ok(None);
            };
            let mut data = read_sample_bytes(&sample)
                .map_err(|error| format!("H.264 출력 읽기 실패: {error}"))?;
            data = normalize_annex_b_owned(data)?;
            let mut nal_units = summarize_annex_b(&data);
            let keyframe =
                unsafe { sample.GetUINT32(&MFSampleExtension_CleanPoint).unwrap_or(0) != 0 }
                    || nal_units.idr;
            let mut has_configuration = nal_units.has_configuration();

            if keyframe && !has_configuration {
                if self.sequence_header.is_empty() {
                    self.sequence_header =
                        output_sequence_header(&self.transform).unwrap_or_default();
                }
                if !self.sequence_header.is_empty() {
                    let mut configured =
                        Vec::with_capacity(self.sequence_header.len() + data.len());
                    configured.extend_from_slice(&self.sequence_header);
                    configured.extend_from_slice(&data);
                    data = configured;
                    nal_units = summarize_annex_b(&data);
                    has_configuration = nal_units.has_configuration();
                }
            }

            let timestamp_us = unsafe { sample.GetSampleTime() }.unwrap_or(0).max(0) as u64 / 10;
            Ok(Some(EncodedAccessUnit {
                data,
                timestamp_us,
                keyframe,
                has_configuration,
            }))
        }
    }

    impl Drop for H264Encoder {
        fn drop(&mut self) {
            let _ = unsafe {
                self.transform
                    .ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0)
                    .and_then(|_| {
                        self.transform
                            .ProcessMessage(MFT_MESSAGE_NOTIFY_END_STREAMING, 0)
                    })
                    .and_then(|_| self.transform.ProcessMessage(MFT_MESSAGE_COMMAND_FLUSH, 0))
            };
            if let Some(activation) = &self.activation {
                let _ = unsafe { activation.ShutdownObject() };
            }
        }
    }

    struct MediaFoundationSession {
        com_initialized: bool,
    }

    impl MediaFoundationSession {
        fn new() -> Result<Self, String> {
            let com_result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let com_initialized = com_result.is_ok();
            if !com_initialized && com_result != windows::Win32::Foundation::RPC_E_CHANGED_MODE {
                return Err(format!("COM 초기화 실패: {com_result:?}"));
            }
            if let Err(error) = unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) } {
                if com_initialized {
                    unsafe { CoUninitialize() };
                }
                return Err(format!("Media Foundation 시작 실패: {error}"));
            }
            Ok(Self { com_initialized })
        }
    }

    impl Drop for MediaFoundationSession {
        fn drop(&mut self) {
            let _ = unsafe { MFShutdown() };
            if self.com_initialized {
                unsafe { CoUninitialize() };
            }
        }
    }

    unsafe fn configure_transform(
        transform: &IMFTransform,
        width: u32,
        height: u32,
        fps: u32,
        bitrate: u32,
    ) -> WindowsResult<()> {
        let attributes = unsafe { transform.GetAttributes()? };
        let asynchronous = unsafe { attributes.GetUINT32(&MF_TRANSFORM_ASYNC) }.unwrap_or(0) != 0;
        if asynchronous {
            unsafe { attributes.SetUINT32(&MF_TRANSFORM_ASYNC_UNLOCK, 1)? };
        }
        let _ = unsafe { attributes.SetUINT32(&MF_LOW_LATENCY, 1) };

        let output_type = unsafe { MFCreateMediaType()? };
        unsafe {
            output_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            output_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            output_type.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
            output_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            output_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_ratio(width, height))?;
            output_type.SetUINT64(&MF_MT_FRAME_RATE, pack_ratio(fps, 1))?;
            output_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_ratio(1, 1))?;
            output_type.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
            output_type.SetUINT32(&MF_MT_MPEG2_LEVEL, eAVEncH264VLevel4_2.0 as u32)?;
            output_type.SetUINT32(&MF_MT_MAX_KEYFRAME_SPACING, fps)?;
            transform.SetOutputType(OUTPUT_STREAM_ID, &output_type, 0)?;
        }

        let input_type = unsafe { MFCreateMediaType()? };
        unsafe {
            input_type.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            input_type.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)?;
            input_type.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
            input_type.SetUINT64(&MF_MT_FRAME_SIZE, pack_ratio(width, height))?;
            input_type.SetUINT64(&MF_MT_FRAME_RATE, pack_ratio(fps, 1))?;
            input_type.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack_ratio(1, 1))?;
            input_type.SetUINT32(&MF_MT_DEFAULT_STRIDE, width)?;
            input_type.SetUINT32(&MF_MT_VIDEO_NOMINAL_RANGE, MFNominalRange_16_235.0 as u32)?;
            transform.SetInputType(INPUT_STREAM_ID, &input_type, 0)?;
        }
        Ok(())
    }

    fn enumerate_hardware_encoders() -> Result<Vec<(IMFActivate, String)>, String> {
        let input = MFT_REGISTER_TYPE_INFO {
            guidMajorType: MFMediaType_Video,
            guidSubtype: MFVideoFormat_NV12,
        };
        let output = MFT_REGISTER_TYPE_INFO {
            guidMajorType: MFMediaType_Video,
            guidSubtype: MFVideoFormat_H264,
        };
        let flags = MFT_ENUM_FLAG_HARDWARE
            | MFT_ENUM_FLAG_ASYNCMFT
            | MFT_ENUM_FLAG_SYNCMFT
            | MFT_ENUM_FLAG_SORTANDFILTER;
        let mut raw_activations: *mut Option<IMFActivate> = ptr::null_mut();
        let mut count = 0_u32;
        unsafe {
            MFTEnumEx(
                MFT_CATEGORY_VIDEO_ENCODER,
                flags,
                Some(&input),
                Some(&output),
                &mut raw_activations,
                &mut count,
            )
            .map_err(|error| format!("하드웨어 H.264 인코더 검색 실패: {error}"))?;
        }

        let mut encoders = Vec::with_capacity(count as usize);
        if !raw_activations.is_null() {
            for index in 0..count as usize {
                let activation = unsafe { ptr::read(raw_activations.add(index)) };
                if let Some(activation) = activation {
                    let name = activation_name(&activation)
                        .unwrap_or_else(|| format!("Hardware H.264 Encoder #{}", index + 1));
                    encoders.push((activation, name));
                }
            }
            unsafe { CoTaskMemFree(Some(raw_activations as *const c_void)) };
        }
        Ok(encoders)
    }

    fn activation_name(activation: &IMFActivate) -> Option<String> {
        let length = unsafe { activation.GetStringLength(&MFT_FRIENDLY_NAME_Attribute) }.ok()?;
        let mut utf16 = vec![0_u16; length as usize + 1];
        unsafe {
            activation
                .GetString(&MFT_FRIENDLY_NAME_Attribute, &mut utf16, None)
                .ok()?;
        }
        Some(String::from_utf16_lossy(&utf16[..length as usize]))
    }

    fn set_codec_value(api: &ICodecAPI, key: *const windows::core::GUID, value: VARIANT) {
        if unsafe { api.IsSupported(key) }.is_ok() {
            let _ = unsafe { api.SetValue(key, &value) };
        }
    }

    fn create_input_sample(
        nv12: &[u8],
        timestamp_100ns: i64,
        duration_100ns: i64,
    ) -> WindowsResult<IMFSample> {
        unsafe {
            let buffer = MFCreateMemoryBuffer(nv12.len() as u32)?;
            let mut destination = ptr::null_mut();
            buffer.Lock(&mut destination, None, None)?;
            ptr::copy_nonoverlapping(nv12.as_ptr(), destination, nv12.len());
            buffer.Unlock()?;
            buffer.SetCurrentLength(nv12.len() as u32)?;

            let sample = MFCreateSample()?;
            sample.AddBuffer(&buffer)?;
            sample.SetSampleTime(timestamp_100ns)?;
            sample.SetSampleDuration(duration_100ns)?;
            Ok(sample)
        }
    }

    fn create_output_sample(capacity: u32) -> WindowsResult<IMFSample> {
        unsafe {
            let sample = MFCreateSample()?;
            let buffer = MFCreateMemoryBuffer(capacity)?;
            sample.AddBuffer(&buffer)?;
            Ok(sample)
        }
    }

    fn read_sample_bytes(sample: &IMFSample) -> WindowsResult<Vec<u8>> {
        unsafe {
            let buffer = sample.ConvertToContiguousBuffer()?;
            let length = buffer.GetCurrentLength()?;
            let mut source = ptr::null_mut();
            buffer.Lock(&mut source, None, None)?;
            let result = slice::from_raw_parts(source, length as usize).to_vec();
            buffer.Unlock()?;
            Ok(result)
        }
    }

    fn output_sequence_header(transform: &IMFTransform) -> WindowsResult<Vec<u8>> {
        unsafe {
            let output_type = transform.GetOutputCurrentType(OUTPUT_STREAM_ID)?;
            let size = output_type.GetBlobSize(&MF_MT_MPEG_SEQUENCE_HEADER)?;
            let mut header = vec![0_u8; size as usize];
            output_type.GetBlob(&MF_MT_MPEG_SEQUENCE_HEADER, &mut header, None)?;
            normalize_annex_b(&header).map_err(|message| {
                windows::core::Error::new(windows::Win32::Foundation::E_FAIL, &message)
            })
        }
    }

    fn pack_ratio(numerator: u32, denominator: u32) -> u64 {
        (u64::from(numerator) << 32) | u64::from(denominator)
    }

    fn normalize_annex_b(bytes: &[u8]) -> Result<Vec<u8>, String> {
        if bytes.is_empty() || starts_with_start_code(bytes) {
            return Ok(bytes.to_vec());
        }
        let mut offset = 0_usize;
        let mut annex_b = Vec::with_capacity(bytes.len() + 32);
        while offset + 4 <= bytes.len() {
            let length = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]) as usize;
            offset += 4;
            if length == 0 || offset + length > bytes.len() {
                return Err("H.264 NAL 길이 형식이 올바르지 않습니다.".to_string());
            }
            annex_b.extend_from_slice(&[0, 0, 0, 1]);
            annex_b.extend_from_slice(&bytes[offset..offset + length]);
            offset += length;
        }
        if offset != bytes.len() || annex_b.is_empty() {
            return Err("H.264 비트스트림을 Annex B로 변환하지 못했습니다.".to_string());
        }
        Ok(annex_b)
    }

    fn normalize_annex_b_owned(bytes: Vec<u8>) -> Result<Vec<u8>, String> {
        if bytes.is_empty() || starts_with_start_code(&bytes) {
            return Ok(bytes);
        }
        normalize_annex_b(&bytes)
    }

    fn starts_with_start_code(bytes: &[u8]) -> bool {
        bytes.starts_with(&[0, 0, 1]) || bytes.starts_with(&[0, 0, 0, 1])
    }

    #[derive(Default)]
    struct NalSummary {
        idr: bool,
        sps: bool,
        pps: bool,
    }

    impl NalSummary {
        fn has_configuration(&self) -> bool {
            self.sps && self.pps
        }
    }

    fn summarize_annex_b(bytes: &[u8]) -> NalSummary {
        let mut summary = NalSummary::default();
        let mut index = 0_usize;
        while index + 3 < bytes.len() {
            let start_length = if bytes[index..].starts_with(&[0, 0, 0, 1]) {
                4
            } else if bytes[index..].starts_with(&[0, 0, 1]) {
                3
            } else {
                index += 1;
                continue;
            };
            let nal_index = index + start_length;
            if nal_index < bytes.len() {
                match bytes[nal_index] & 0x1f {
                    5 => summary.idr = true,
                    7 => summary.sps = true,
                    8 => summary.pps = true,
                    _ => {}
                }
            }
            index = nal_index.saturating_add(1);
        }
        summary
    }

    #[cfg(test)]
    fn annex_b_nal_types(bytes: &[u8]) -> Vec<u8> {
        let mut types = Vec::new();
        let mut index = 0_usize;
        while index + 3 < bytes.len() {
            let start_length = if bytes[index..].starts_with(&[0, 0, 0, 1]) {
                4
            } else if bytes[index..].starts_with(&[0, 0, 1]) {
                3
            } else {
                index += 1;
                continue;
            };
            let nal_index = index + start_length;
            if nal_index < bytes.len() {
                types.push(bytes[nal_index] & 0x1f);
            }
            index = nal_index.saturating_add(1);
        }
        types
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn converts_length_prefixed_nalus_to_annex_b() {
            let avcc = [0, 0, 0, 2, 0x67, 1, 0, 0, 0, 2, 0x68, 2];
            assert_eq!(
                normalize_annex_b(&avcc).unwrap(),
                [0, 0, 0, 1, 0x67, 1, 0, 0, 0, 1, 0x68, 2]
            );
        }

        #[test]
        fn extracts_annex_b_nal_types() {
            let stream = [0, 0, 0, 1, 0x67, 1, 0, 0, 1, 0x68, 2, 0, 0, 1, 0x65, 3];
            assert_eq!(annex_b_nal_types(&stream), vec![7, 8, 5]);
        }

        #[test]
        fn media_foundation_encodes_synthetic_nv12_when_requested() {
            if std::env::var_os("HEART_TEST_H264").is_none() {
                return;
            }
            let width = 1_728_u32;
            let height = 1_080_u32;
            let frame = vec![128_u8; width as usize * height as usize * 3 / 2];
            let mut encoder = H264Encoder::new(width, height, 60, 18_000_000)
                .expect("initialize Media Foundation H.264 encoder");
            eprintln!(
                "HEART H.264 test encoder: {} (hardware={})",
                encoder.encoder_name(),
                encoder.is_hardware()
            );
            let mut access_units = Vec::new();
            for frame_index in 0..90_i64 {
                access_units.extend(
                    encoder
                        .encode(&frame, frame_index * HUNDRED_NS_PER_SECOND / 60)
                        .expect("encode synthetic NV12 frame"),
                );
                if access_units.iter().any(|unit| unit.keyframe) {
                    break;
                }
            }
            access_units.extend(encoder.poll().expect("drain H.264 output"));
            assert!(
                access_units.iter().any(|unit| unit.keyframe),
                "encoder did not produce a key frame"
            );
            assert!(
                access_units
                    .iter()
                    .any(|unit| starts_with_start_code(&unit.data)),
                "encoder output is not Annex B"
            );
            assert!(
                access_units.iter().any(|unit| unit.has_configuration),
                "key frame did not include SPS/PPS configuration"
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows_encoder::*;

#[cfg(not(target_os = "windows"))]
mod unsupported {
    #[derive(Debug, Clone)]
    pub struct EncodedAccessUnit {
        pub data: Vec<u8>,
        pub timestamp_us: u64,
        pub keyframe: bool,
        pub has_configuration: bool,
    }

    pub struct H264Encoder;

    impl H264Encoder {
        pub fn new(_: u32, _: u32, _: u32, _: u32) -> Result<Self, String> {
            Err("H.264 하드웨어 인코딩은 Windows에서만 지원합니다.".to_string())
        }

        pub fn encoder_name(&self) -> &str {
            "unavailable"
        }

        pub fn is_hardware(&self) -> bool {
            false
        }

        pub fn force_keyframe(&self) {}

        pub fn encode(&mut self, _: &[u8], _: i64) -> Result<Vec<EncodedAccessUnit>, String> {
            Err("H.264 하드웨어 인코딩은 Windows에서만 지원합니다.".to_string())
        }

        pub fn poll(&mut self) -> Result<Vec<EncodedAccessUnit>, String> {
            Err("H.264 하드웨어 인코딩은 Windows에서만 지원합니다.".to_string())
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub use unsupported::*;
