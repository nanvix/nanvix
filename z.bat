@echo off
REM Copyright(c) The Maintainers of Nanvix.
REM Licensed under the MIT License.
REM Thin wrapper that delegates to the unified Python build backend (z.py).
setlocal

REM Prefer the Python launcher with an explicit 3.10+ requirement.
where py >nul 2>&1
if %ERRORLEVEL%==0 (
    py -3.10 "%~dp0z.py" %*
    set "EXITCODE=%ERRORLEVEL%"
    goto :end
)

REM Fallback to python on PATH.
where python >nul 2>&1
if %ERRORLEVEL%==0 (
    python "%~dp0z.py" %*
    set "EXITCODE=%ERRORLEVEL%"
    goto :end
)

REM No suitable Python interpreter found.
>&2 echo [z.bat] Error: Python 3.10 or newer is required to run z.py.
>&2 echo [z.bat] Install Python 3.10+ and ensure it is available on PATH.
set "EXITCODE=1"

:end
endlocal & exit /b %EXITCODE%
