# Tabula Blockchain - Tauri Multi-Platform App

A modern blockchain application built with Rust and Tauri v2, featuring a beautiful web-based UI and support for **desktop and mobile platforms**.

## 🎯 Supported Platforms

- ✅ **macOS** (Desktop)
- ✅ **Windows** (Desktop)
- ✅ **Linux** (Desktop)
- ✅ **iOS** (Mobile)
- ✅ **Android** (Mobile)

## 📦 Features

- **Block Structure**: Index, timestamp, data, previous hash, current hash, and nonce
- **SHA-256 Hashing**: Cryptographic hashing for data integrity
- **Proof of Work**: Mining algorithm with configurable difficulty
- **Chain Validation**: Verify blockchain integrity
- **Beautiful UI**: Modern, responsive web interface
- **Cross-Platform**: Single codebase for all platforms

## 🚀 Quick Start (Desktop)

### Prerequisites

- Rust (1.70 or later) - [Install from rustup.rs](https://rustup.rs/)
- Node.js is NOT required (we're using static HTML/CSS/JS)

### Running the Desktop App

```bash
# Navigate to src-tauri directory
cd src-tauri

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

The app will open in a native window with the blockchain interface!

### Desktop Outputs

After building, you'll find installers in `src-tauri/target/release/bundle/`:

- **macOS**: `.dmg` file in `dmg/` folder
- **Windows**: `.msi` installer in `msi/` folder  
- **Linux**: `.deb`, `.AppImage`, or `.rpm` in respective folders

## 📱 Mobile Setup

### iOS Setup

**Requirements:**
- macOS computer
- Xcode (from App Store)
- Apple Developer account (for physical devices)

**Initialize iOS:**
```bash
cd src-tauri
cargo tauri ios init
```

**Run on iOS Simulator:**
```bash
cargo tauri ios dev
```

**Build for iOS:**
```bash
cargo tauri ios build
# Creates .ipa file in gen/apple/build/
```

### Android Setup

**Requirements:**
- Android Studio
- Android SDK & NDK
- Java JDK 17+

**Initialize Android:**
```bash
cd src-tauri
cargo tauri android init
```

**Run on Android Emulator:**
```bash
cargo tauri android dev
```

**Build for Android:**
```bash
cargo tauri android build
# Creates .apk/.aab in gen/android/app/build/outputs/
```

## 🎨 UI Features

The application includes:

- **Dashboard**: View blockchain statistics
- **Block Visualization**: See all blocks with hashes and data
- **Add Blocks**: Mine new blocks with custom data
- **Real-time Validation**: Check chain integrity
- **Mining Progress**: Visual feedback during mining
- **Responsive Design**: Works on desktop and mobile screens

## 🏗️ Project Structure

```
tabula/core/
├── src/                      # Rust library (blockchain logic)
│   ├── lib.rs
│   ├── blockchain.rs         # Core blockchain implementation
│   └── main.rs               # CLI version
├── src-tauri/                # Tauri backend
│   ├── src/main.rs           # Tauri commands & app entry
│   ├── Cargo.toml
│   ├── tauri.conf.json       # Platform configuration
│   └── icons/                # App icons
├── ui/                       # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── styles.css
│   └── app.js
├── gen/                      # Generated (after mobile init)
│   ├── android/              # Android Studio project
│   └── apple/                # Xcode project
└── Cargo.toml
```

## 🔧 Development

### Run CLI Version

```bash
# From project root
cargo run
```

### Run Tauri Desktop App

```bash
cd src-tauri
cargo tauri dev
```

### Test on Mobile

```bash
# iOS Simulator
cd src-tauri
cargo tauri ios dev

# Android Emulator
cargo tauri android dev
```

## 📝 API Commands

The Tauri backend exposes these commands to the frontend:

- `get_blockchain()` - Returns the entire blockchain
- `add_block(data: String)` - Mines and adds a new block
- `validate_chain()` - Checks if the blockchain is valid
- `get_difficulty()` - Returns the current mining difficulty
- `greet(name: String)` - Test command

## 🎯 Customization

### Change Mining Difficulty

Edit `src-tauri/src/main.rs`:

```rust
let state = AppState::new(2); // Change 2 to desired difficulty
```

Higher difficulty = more leading zeros required in hash = harder to mine.

### Modify UI

Edit files in the `ui/` directory:
- `index.html` - Structure
- `styles.css` - Styling
- `app.js` - Logic and Tauri API calls

### Add Features

1. **Backend**: Add functions in `src/blockchain.rs`
2. **Tauri Commands**: Expose them in `src-tauri/src/main.rs`
3. **Frontend**: Call them from `ui/app.js` using `invoke()`

## 🔐 Code Signing & Distribution

### macOS

```bash
# Sign the app
codesign --deep --force --verify --verbose --sign "Developer ID" app.app

# Notarize for macOS 10.15+
xcrun notarytool submit app.dmg --apple-id you@email.com --team-id TEAMID
```

### iOS

- Requires Apple Developer Program ($99/year)
- Configure signing in Xcode project at `gen/apple/`

### Android

```bash
# Generate keystore
keytool -genkey -v -keystore release.keystore -alias tabula -keyalg RSA

# Sign APK
jarsigner -verbose -sigalg SHA256withRSA -digestalg SHA-256 \
  -keystore release.keystore app.apk tabula
```

## 🐛 Troubleshooting

### Icons Missing

Generate proper icons:
```bash
cd src-tauri
cargo tauri icon path/to/icon.png
```

### Mobile Build Fails

Make sure you've initialized mobile targets:
```bash
cargo tauri ios init
cargo tauri android init
```

### App Won't Start

Check logs:
```bash
# Desktop
cargo tauri dev --verbose

# iOS
cargo tauri ios dev --verbose

# Android  
cargo tauri android dev --verbose
```

## 📚 Learn More

- [Tauri Documentation](https://v2.tauri.app/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Blockchain Concepts](https://en.wikipedia.org/wiki/Blockchain)

## 📄 License

This project is open source and available for educational purposes.

---

Built with ❤️ using Rust 🦀 and Tauri
