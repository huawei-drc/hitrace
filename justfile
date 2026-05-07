import 'emulator-action/justfile'

# Run the OHOS hitrace smoke test against a connected emulator/device.
test:
    cargo xtask ohos-trace-smoke
