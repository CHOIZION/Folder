# HEART

Windows용 로컬 미디어 라이브러리와 Android 리모컨입니다. 영상, 이미지, 만화, 게임, 문서를 한곳에서 정리하고 휴대폰에서 검색·실행·화면 조작을 할 수 있습니다.

이 저장소는 **1.4.2 공개용 소스**입니다. 사용자 미디어, 데이터베이스, 접속 정보, 설치 파일은 포함하지 않습니다. 인터페이스는 한국어이며, 데스크톱은 Windows를 대상으로 합니다.

## 기능

- 폴더 스캔, 변경 감지, 계층형 카테고리와 사용자 카테고리
- 즐겨찾기, 별점, 메모, 태그 자동완성, 여러 조건의 AND 검색
- 기본 프로그램으로 미디어 열기, 게임 실행, 영상별 재생
- 최근 실행 목록, 통계, 무작위 선택, 마우스 가운데 버튼 메뉴
- 한국어 음성 명령과 시간 기반 감상 목록 구성
- 선택적으로 사용하는 Android 원격 검색·실행·키보드·마우스 조작
- Windows 화면 전송: H.264 / Android MediaCodec, MJPEG 호환 경로
- 트레이 최소화와 로컬 SQLite 저장

음성 명령과 감상 목록은 규칙 기반 기능이며 LLM API 키가 필요하지 않습니다. 음성 인식은 실행 환경의 Web Speech API 지원에 따라 동작하고, 해당 환경이 외부 음성 서비스를 이용할 수 있습니다.

## 개발 환경

- Windows 10/11, WebView2, Visual Studio C++ Build Tools 및 Windows SDK
- Node.js 24 이상과 npm
- Rust stable MSVC 도구 모음 (`rustfmt`, `clippy` 포함)
- Android 빌드에만 JDK 17, Android SDK Platform 35, Build Tools 35.0.1 필요

```powershell
npm ci
npm run tauri dev
```

웹 화면만 확인하려면 `npm run dev`를 사용합니다. 파일 스캔·데이터베이스·원격 호스트는 Tauri 실행 환경이 필요합니다.

## 라이브러리 시작

1. 상단의 **라이브러리 폴더 경로 입력**에 사용할 폴더의 절대 경로를 입력합니다.
2. **열기**를 누르면 스캔하고 선택한 경로를 이 앱의 로컬 저장소에 기억합니다.
3. 이후 **다시 스캔**으로 갱신할 수 있으며 실행 중 파일 변경도 감지합니다.

최초 실행은 빈 상태로 시작합니다. 폴더를 직접 선택하기 전에는 개인 미디어를 자동으로 검색하지 않습니다.

권장 폴더 이름은 `영상`, `이미지`, `만화`, `게임`, `문서`입니다. `Videos`, `Images`, `Comics`, `Games`, `Documents`도 지원합니다. 다른 폴더 이름은 일반 분류 규칙을 적용하며 사용자 카테고리를 별도로 만들 수 있습니다. 이미지 뷰어 Honeyview가 표준 경로에 있으면 사용하고, 없으면 Windows 기본 연결 프로그램을 사용합니다.

검색 예시: `topic:Travel, genre:Documentary`. 쉼표로 나눈 조건을 모두 만족하는 항목을 찾습니다. `creator:Example`처럼 사용자 정의 태그 접두사도 검색할 수 있습니다. 카드·통계에서는 `genre:`를 장르, `topic:`을 주제, 그 외 태그를 기타로 표시합니다.

## 휴대폰 연결

원격 호스트는 기본으로 꺼져 있습니다. 사용하려면 **앱을 완전히 종료한 뒤**, PowerShell에서 다음과 같이 실행합니다.

```powershell
$env:HEART_ENABLE_REMOTE = "1"
npm run tauri dev
```

설치된 앱은 같은 PowerShell에서 `$env:HEART_ENABLE_REMOTE = "1"`을 설정한 다음 앱 실행 파일을 실행합니다. 끄려면 앱을 종료하고 `Remove-Item Env:HEART_ENABLE_REMOTE -ErrorAction SilentlyContinue` 후 다시 실행합니다. 트레이에 숨겨진 앱도 완전히 종료해야 합니다.

1. PC의 **REMOTE** 패널에서 주소와 6자리 코드를 확인합니다.
2. [Android 앱](android-remote/README.md)을 빌드해 설치합니다.
3. 같은 Wi-Fi에서는 자동 검색, 사설 VPN에서는 주소 입력을 사용합니다.
4. 코드를 입력해 페어링합니다. 연결 초기화나 PC 앱 재시작 시 기존 토큰은 무효화됩니다.

원격 연결은 전체 화면과 PC 입력을 다룹니다. 구현은 HTTP이며 자체 TLS 암호화를 제공하지 않습니다. 신뢰하는 LAN 또는 암호화된 사설 VPN에서 사용하고 인터넷에 포트를 직접 공개하지 마세요. TCP 37218–37228 중 사용 가능한 포트와 UDP 37219를 사용합니다. 자동 검색은 LAN 브로드캐스트라 사설 VPN 구간에서는 직접 주소를 입력해야 합니다. 스트리밍의 실제 프레임률은 PC·휴대폰·네트워크에 따라 달라집니다.

## 데이터와 개인정보

SQLite DB는 OS가 제공하는 `org.heartlibrary.heart` 앱 데이터 폴더의 `heart.db`에 저장됩니다. `HEART_DATA_DIR` 환경 변수로 테스트용 경로를 지정할 수 있습니다. DB와 메모는 암호화 저장소가 아닙니다. 원격 주소·토큰은 Android 앱의 비공개 설정에 저장됩니다.

이 공개판은 별도 앱 식별자를 사용하므로 이전 설치본의 DB나 Android 연결 정보를 자동으로 가져오지 않습니다. 기존 데이터는 별도로 보관하세요. 실행 후 생긴 DB, 미디어, 로그, 스크린샷을 이 저장소에 추가하지 마세요.

## 검증과 빌드

```powershell
npm run verify
npm run build
npm run tauri build
```

`verify`는 소스 공개 검사, JavaScript 테스트, Svelte/TypeScript 검사, Rust 포맷·Clippy·테스트를 실행합니다. Windows CI는 웹 빌드와 Android 테스트 APK 빌드도 수행합니다. 실제 실행 파일 빌드는 위 마지막 명령으로 수행합니다.

구조는 [ARCHITECTURE.md](ARCHITECTURE.md), 기여 방법은 [CONTRIBUTING.md](CONTRIBUTING.md), 보안 범위는 [SECURITY.md](SECURITY.md)를 참고하세요.

## 라이선스

프로젝트 소스와 새 기하학 아이콘은 [MIT](LICENSE)입니다. 의존성에는 각각의 라이선스가 적용됩니다. [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)를 참고하세요.
