[CmdletBinding()]
param(
    [string]$AndroidSdkRoot = $env:ANDROID_HOME,
    [string]$JdkHome = $env:JAVA_HOME
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $AndroidSdkRoot -or -not $JdkHome) {
    throw 'Set AndroidSdkRoot and JdkHome (or ANDROID_HOME and JAVA_HOME).'
}
$buildTools = Join-Path $AndroidSdkRoot 'build-tools\35.0.1'
$androidJar = Join-Path $AndroidSdkRoot 'platforms\android-35\android.jar'
$javac = Join-Path $JdkHome 'bin\javac.exe'
$jar = Join-Path $JdkHome 'bin\jar.exe'
$keytool = Join-Path $JdkHome 'bin\keytool.exe'
$aapt2 = Join-Path $buildTools 'aapt2.exe'
$d8 = Join-Path $buildTools 'd8.bat'
$zipalign = Join-Path $buildTools 'zipalign.exe'
$apksigner = Join-Path $buildTools 'apksigner.bat'
foreach ($required in @($androidJar, $javac, $jar, $keytool, $aapt2, $d8, $zipalign, $apksigner)) {
    if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing tool: $required" }
}
function Invoke-Checked([string]$Tool, [string[]]$ToolArgs) {
    & $Tool @ToolArgs
    if ($LASTEXITCODE -ne 0) { throw "Build tool failed: $Tool ($LASTEXITCODE)" }
}
$outputRoot = Join-Path $PSScriptRoot 'build'
# Each run uses a new intermediate directory; no recursive cleanup is needed.
$stage = Join-Path $outputRoot ([guid]::NewGuid().ToString('N'))
$classes = Join-Path $stage 'classes'
$generated = Join-Path $stage 'generated'
$dex = Join-Path $stage 'dex'
New-Item -ItemType Directory -Force -Path $outputRoot, $stage, $classes, $generated, $dex | Out-Null
$resources = Join-Path $stage 'resources.zip'
$unsigned = Join-Path $stage 'unsigned.apk'
$aligned = Join-Path $stage 'aligned.apk'
$classJar = Join-Path $stage 'classes.jar'
$apk = Join-Path $outputRoot 'HEART-Remote-debug.apk'
$key = Join-Path $outputRoot 'debug.keystore'
$oldJavaHome = $env:JAVA_HOME
try {
    $env:JAVA_HOME = $JdkHome
    Invoke-Checked $aapt2 @('compile', '--dir', (Join-Path $PSScriptRoot 'res'), '-o', $resources)
    Invoke-Checked $aapt2 @('link', '-o', $unsigned, '-I', $androidJar, '--manifest', (Join-Path $PSScriptRoot 'AndroidManifest.xml'), '--java', $generated, '--min-sdk-version', '26', '--target-sdk-version', '35', $resources)
    $sources = @(Get-ChildItem -LiteralPath (Join-Path $PSScriptRoot 'src'), $generated -Filter '*.java' -Recurse -File)
    $sourceList = Join-Path $stage 'sources.txt'
    $sourceLines = $sources | ForEach-Object { '"' + $_.FullName.Replace('\', '/') + '"' }
    [IO.File]::WriteAllLines($sourceList, [string[]]$sourceLines, [Text.UTF8Encoding]::new($false))
    Invoke-Checked $javac @('-encoding', 'UTF-8', '--release', '8', '-classpath', $androidJar, '-d', $classes, "@$sourceList")
    Invoke-Checked $jar @('cf', $classJar, '-C', $classes, '.')
    Invoke-Checked $d8 @('--lib', $androidJar, '--min-api', '26', '--output', $dex, $classJar)
    $dexFiles = @(Get-ChildItem -LiteralPath $dex -Filter '*.dex' -File)
    foreach ($dexFile in $dexFiles) {
        Invoke-Checked $jar @('uf', $unsigned, '-C', $dex, $dexFile.Name)
    }
    Invoke-Checked $zipalign @('-f', '4', $unsigned, $aligned)
    if (-not (Test-Path -LiteralPath $key)) {
        # Standard public debug credentials; never use this key for a release.
        Invoke-Checked $keytool @('-genkeypair', '-keystore', $key, '-storepass', 'android', '-keypass', 'android', '-alias', 'androiddebugkey', '-keyalg', 'RSA', '-keysize', '2048', '-validity', '10000', '-dname', 'CN=Android Debug,O=Android,C=US', '-noprompt')
    }
    Invoke-Checked $apksigner @('sign', '--ks', $key, '--ks-key-alias', 'androiddebugkey', '--ks-pass', 'pass:android', '--key-pass', 'pass:android', '--out', $apk, $aligned)
    Invoke-Checked $apksigner @('verify', '--verbose', $apk)
    Write-Output "Built test APK: $apk"
} finally {
    $env:JAVA_HOME = $oldJavaHome
}
