# HEART Remote for Android

Android 8.0(API 26) 이상용 라이브러리 브라우저와 Windows 터치 리모컨입니다. AndroidX나 외부 Java 라이브러리 없이 Android SDK API를 사용합니다.

## 빌드

Windows PowerShell, JDK 17, Android Platform 35, Build Tools 35.0.1이 필요합니다.
저장소 루트에서 실행합니다. SDK와 JDK 경로는 자신의 환경에 맞게 지정하세요.

```powershell
./android-remote/build-apk.ps1 -AndroidSdkRoot $env:ANDROID_HOME -JdkHome $env:JAVA_HOME
```

`android-remote/build/HEART-Remote-debug.apk`를 만듭니다. 빌드 폴더에 로컬 테스트용 키를 생성하며 저장소에 포함하지 않습니다. 이 APK는 테스트용으로 서명됩니다. 스토어 배포용 서명이나 자동 게시 기능은 제공하지 않습니다.

```powershell
adb install -r ./android-remote/build/HEART-Remote-debug.apk
```

공개판 패키지 ID는 `org.heartlibrary.heartremote`입니다. 이전 개인용 앱과 별도로 설치되며 페어링 정보를 가져오지 않습니다.

## 연결과 조작

1. PC에서 `HEART_ENABLE_REMOTE=1`을 설정하고 HEART를 시작합니다.
2. PC의 REMOTE 패널에서 주소와 6자리 코드를 확인합니다.
3. Android 앱에서 같은 Wi-Fi 자동 검색 또는 주소 입력으로 연결합니다.
4. 코드를 입력해 페어링합니다. 사설 VPN은 주소를 직접 입력합니다.

신뢰하는 LAN 또는 암호화된 사설 VPN에서 사용하세요. 원격 호스트는 자체 TLS를 제공하지 않으며 인터넷에 직접 노출하는 용도가 아닙니다.

- 짧게 터치: 클릭
- 빠른 좌우 스와이프: 방향키, 상하 스와이프: 세로 스크롤
- 길게 누르고 이동: 마우스 이동·드래그
- 두 손가락 이동: 가로·세로 스크롤
- 전체 화면: 가로 몰입 화면, 뒤로가기: 복귀
- 문자 입력: 휴대폰 키보드로 PC에 입력

H.264/MediaCodec 경로와 MJPEG 호환 경로를 유지합니다. 실제 프레임률은 장치와 연결 상태에 따라 달라집니다. 앱을 백그라운드로 보내면 스트리밍 자원을 해제합니다.

## 구조

`src/org/heartlibrary/heartremote`에 연결 화면, HTTP/UDP 클라이언트, WebView, H.264 수신, 제스처·입력 처리가 분리되어 있습니다. 서명 키·APK·SDK·연결 정보는 커밋하지 마세요.
