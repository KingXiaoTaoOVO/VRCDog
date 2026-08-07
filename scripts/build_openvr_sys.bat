@echo off
call "C:\Program Files\Microsoft Visual Studio\18\Community\VC\Auxiliary\Build\vcvarsall.bat" x64
if errorlevel 1 (
  echo VCVARSALL_FAILED > "C:\Users\Administrator\Documents\Project\VRCDog\scripts\openvr_build.log"
  exit /b 1
)
set "PATH=C:\Program Files\Microsoft Visual Studio\18\Community\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;C:\Users\Administrator\.cargo\bin;%PATH%"
cd /d C:\Users\Administrator\Documents\Project\VRCDog\src-tauri
echo BUILD_START > "C:\Users\Administrator\Documents\Project\VRCDog\scripts\openvr_build.log"
cargo build -p openvr_sys -j 1 >> "C:\Users\Administrator\Documents\Project\VRCDog\scripts\openvr_build.log" 2>&1
echo BUILD_EXIT=%ERRORLEVEL% >> "C:\Users\Administrator\Documents\Project\VRCDog\scripts\openvr_build.log"
