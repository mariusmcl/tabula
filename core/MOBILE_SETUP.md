# 📱 Mobile Setup Guide

This guide will help you set up and run your Tabula Blockchain app on iOS and Android devices.

## 🍎 iOS Setup

### Prerequisites

1. **macOS Computer** (required for iOS development)
2. **Xcode** - Install from Mac App Store (free)
3. **Xcode Command Line Tools**:
   ```bash
   xcode-select --install
   ```
4. **Apple Developer Account** (optional for simulator, required for physical devices)

### Initialize iOS Project

```bash
cd src-tauri
cargo tauri ios init
```

This will:
- Create `gen/apple/` directory with Xcode project
- Set up iOS build configuration
- Generate necessary mobile bindings

### Run on iOS Simulator

```bash
# List available simulators
xcrun simctl list devices

# Run app (will open simulator automatically)
cargo tauri ios dev

# Or specify a specific simulator
cargo tauri ios dev --device "iPhone 16"
```

### Build for iOS Device

```bash
# Development build
cargo tauri ios build

# Release build
cargo tauri ios build --release

# Output: gen/apple/build/Release-iphoneos/tabula_app.ipa
```

### Deploy to Physical iPhone/iPad

1. Open Xcode project:
   ```bash
   open gen/apple/tabula_app.xcodeproj
   ```

2. In Xcode:
   - Select your device from the top bar
   - Go to "Signing & Capabilities" tab
   - Select your Apple Developer Team
   - Click Run (▶️)

### Troubleshooting iOS

**Simulator not opening:**
```bash
open -a Simulator
```

**Build errors:**
```bash
# Clean build
cd gen/apple
xcodebuild clean
cd ../..
cargo tauri ios build
```

**Signing issues:**
- Need Apple Developer account for physical devices
- Simulator doesn't require signing

---

## 🤖 Android Setup

### Prerequisites

1. **Android Studio** - [Download here](https://developer.android.com/studio)
2. **Android SDK** (installed with Android Studio)
3. **Android NDK** (will be installed during init)
4. **Java JDK 17+**:
   ```bash
   # Check Java version
   java --version
   
   # Install if needed (macOS)
   brew install openjdk@17
   ```

### Set Environment Variables

Add to your `~/.zshrc` or `~/.bash_profile`:

```bash
export JAVA_HOME=/Library/Java/JavaVirtualMachines/jdk-17.jdk/Contents/Home
export ANDROID_HOME=$HOME/Library/Android/sdk
export NDK_HOME=$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk)
export PATH=$PATH:$ANDROID_HOME/emulator
export PATH=$PATH:$ANDROID_HOME/tools
export PATH=$PATH:$ANDROID_HOME/tools/bin
export PATH=$PATH:$ANDROID_HOME/platform-tools
```

Then reload:
```bash
source ~/.zshrc
```

### Initialize Android Project

```bash
cd src-tauri
cargo tauri android init
```

This will:
- Create `gen/android/` directory with Android Studio project
- Install required Android SDK/NDK components
- Set up Gradle build configuration

### Run on Android Emulator

```bash
# List available emulators
emulator -list-avds

# Start an emulator (if not already running)
emulator -avd Pixel_5_API_34 &

# Run app
cargo tauri android dev

# Or specify device
cargo tauri android dev --device emulator-5554
```

### Build Android APK

```bash
# Debug APK
cargo tauri android build

# Release APK (signed)
cargo tauri android build --release

# Output: gen/android/app/build/outputs/apk/
```

### Create Android Emulator

If you don't have an emulator:

```bash
# Open Android Studio
# Tools → Device Manager → Create Device
# Choose: Pixel 5, Android 14 (API 34)
```

Or via command line:
```bash
# List available system images
sdkmanager --list

# Install system image
sdkmanager "system-images;android-34;google_apis;arm64-v8a"

# Create AVD
avdmanager create avd -n Pixel_5_API_34 \
  -k "system-images;android-34;google_apis;arm64-v8a" \
  -d pixel_5
```

### Deploy to Physical Android Device

1. Enable Developer Options on your Android device:
   - Settings → About Phone → Tap "Build Number" 7 times

2. Enable USB Debugging:
   - Settings → Developer Options → USB Debugging

3. Connect device via USB

4. Run:
   ```bash
   # Check device is connected
   adb devices
   
   # Run app
   cargo tauri android dev
   ```

### Troubleshooting Android

**NDK not found:**
```bash
# Install NDK via Android Studio:
# Tools → SDK Manager → SDK Tools → NDK (Side by side)
```

**Gradle errors:**
```bash
cd gen/android
./gradlew clean
cd ../..
cargo tauri android build
```

**Device not detected:**
```bash
# Restart ADB
adb kill-server
adb start-server
adb devices
```

---

## 🔧 Mobile Development Tips

### Hot Reload

Both iOS and Android support hot reload during development:
- Make changes to your Rust code or UI files
- The app will automatically rebuild and reload
- Changes appear in seconds!

### Debugging

**iOS:**
```bash
# View logs
cargo tauri ios dev --verbose

# Or in Xcode: View → Debug Area → Show Debug Area
```

**Android:**
```bash
# View logs
adb logcat | grep RustStdout

# Or in Android Studio: View → Tool Windows → Logcat
```

### Testing on Real Devices

**Why test on real devices:**
- Different screen sizes and resolutions
- Real-world performance
- Touch gestures feel different
- Network conditions vary
- Battery usage

**Recommended test devices:**
- iOS: iPhone SE (small), iPhone 16 (standard), iPad
- Android: Budget phone (performance), Flagship (features)

### Mobile-Specific Considerations

**Performance:**
- Mining blocks is CPU-intensive
- Consider lighter difficulty on mobile
- Show "Mining..." indicators
- Use background threads

**Storage:**
- Mobile has limited storage
- Consider blockchain pruning
- Cache management

**Network:**
- Handle offline gracefully
- Sync when connected
- Progress indicators

**UI/UX:**
- Touch-friendly tap targets (44pt minimum)
- Swipe gestures
- Pull-to-refresh
- Native navigation patterns

---

## 📦 Building for Production

### iOS App Store

1. Set up app signing in Xcode
2. Archive the app: Product → Archive
3. Upload to App Store Connect
4. Submit for review

**Requirements:**
- Apple Developer Program ($99/year)
- App icons (all sizes)
- Screenshots
- Privacy policy
- App description

### Google Play Store

1. Create signed APK/AAB:
   ```bash
   cargo tauri android build --release
   ```

2. Sign the APK (if not auto-signed):
   ```bash
   # Generate keystore (one time)
   keytool -genkey -v -keystore release.keystore \
     -alias tabula -keyalg RSA -keysize 2048 -validity 10000
   
   # Sign APK
   jarsigner -verbose -sigalg SHA256withRSA \
     -digestalg SHA-256 -keystore release.keystore \
     app-release-unsigned.apk tabula
   ```

3. Upload to Google Play Console
4. Fill in store listing
5. Submit for review

**Requirements:**
- Google Play Developer account ($25 one-time)
- App icons
- Feature graphic
- Screenshots
- Privacy policy

---

## 🎯 Next Steps

1. **Initialize iOS**: `cargo tauri ios init`
2. **Initialize Android**: `cargo tauri android init`
3. **Test on simulator/emulator**: `cargo tauri [ios|android] dev`
4. **Make mobile-optimized UI changes**
5. **Test on physical devices**
6. **Build and distribute**

## 📚 Resources

- [Tauri Mobile Documentation](https://v2.tauri.app/develop/mobile/)
- [iOS Developer](https://developer.apple.com/ios/)
- [Android Developer](https://developer.android.com/)
- [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- [Google Play Policy](https://play.google.com/about/developer-content-policy/)

---

Need help? Check the main README.md for troubleshooting tips!

