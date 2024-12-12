:: https://vulkan.lunarg.com/doc/view/1.3.250.1/windows/khronos_validation_layer.html#user-content-layer-details
:: VK_LAYER_ENABLES += VALIDATION_CHECK_ENABLE_SYNCHRONIZATION_VALIDATION_QUEUE_SUBMIT

:: https://vulkan.lunarg.com/doc/sdk/1.3.296.0/windows/crash_diagnostic_layer.html
:: put build output (json + dll) in vk_crash_diagnostics

@ECHO off

SET YY=%date:~6,4%
SET MM=%date:~3,2%
SET DD=%date:~0,2%

SET HH=%time:~0,2%
IF %HH% lss 10 (SET HH=0%time:~1,1%)
SET NN=%time:~3,2%
SET SS=%time:~6,2%
SET MS=%time:~9,2%

SET TIMETAG=%YY%_%MM%_%DD%-%HH%_%NN%_%SS%_%MS%

@ECHO on

set VK_ADD_LAYER_PATH=vk_crash_diagnostics
set VK_INSTANCE_LAYERS=VK_LAYER_LUNARG_api_dump;VK_LAYER_KHRONOS_validation;VK_LAYER_LUNARG_crash_diagnostic
set VK_APIDUMP_LOG_FILENAME=dumps/api_dump_%TIMETAG%.txt
set VK_LAYER_ENABLES=VK_VALIDATION_FEATURE_ENABLE_SYNCHRONIZATION_VALIDATION_EXT
set CDL_LOG_FILE=dumps/crash_dump_%TIMETAG%.txt
set RUST_LOG=trace
mkdir dumps 2> NUL
[program name] > "dumps/log_dump_%TIMETAG%.txt" 2>&1
pause
