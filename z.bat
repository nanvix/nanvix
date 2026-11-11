@echo off
REM Copyright(c) The Maintainers of Nanvix.
REM Licensed under the MIT License.
REM Thin wrapper that delegates to the unified Python build backend (z.py).
setlocal

REM Exits with a nonzero status on Python versions older than 3.10. Keep in sync with z.ps1.
set "VERSION_CHECK=import sys; sys.exit(sys.version_info < (3, 10))"

REM Prefer the Python launcher. Note that 'py -3' selects the newest installed Python 3, whereas
REM 'py -3.10' selects Python 3.10 only.
set "PYTHON=py -3"
where py >nul 2>&1 || goto :try_python
%PYTHON% -c "%VERSION_CHECK%" >nul 2>&1 && goto :run

:try_python
REM Fall back to python on PATH.
set "PYTHON=python"
where python >nul 2>&1 || goto :no_python
%PYTHON% -c "%VERSION_CHECK%" >nul 2>&1 && goto :run

:no_python
>&2 echo [z.bat] Error: Python 3.10 or newer is required to run z.py.
>&2 echo [z.bat] Install Python 3.10+ and ensure it is available on PATH.
exit /b 1

:run
REM Expand ERRORLEVEL on its own line, after z.py returns, so that its exit status is preserved.
%PYTHON% "%~dp0z.py" %*
exit /b %ERRORLEVEL%
