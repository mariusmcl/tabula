# 🎨 Adding Icons to Your Tauri App

Currently, the app runs without custom icons (uses default Tauri icon). Here's how to add your own!

## Quick Setup (Easiest)

### 1. Create or Download an Icon

You need a **1024x1024 PNG image** with transparency. You can:
- Design one in Figma/Sketch/Photoshop
- Use an AI generator (DALL-E, Midjourney)
- Download from [Flaticon](https://www.flaticon.com/) or [Icons8](https://icons8.com/)
- Use a blockchain-themed icon 🔗⛓️

### 2. Generate All Icon Sizes

Tauri includes a built-in icon generator:

```bash
cd src-tauri
cargo tauri icon /path/to/your/icon.png
```

This automatically generates:
- `icons/32x32.png`
- `icons/128x128.png`
- `icons/128x128@2x.png`
- `icons/icon.icns` (macOS)
- `icons/icon.ico` (Windows)
- `icons/icon.png` (Linux)

### 3. Update tauri.conf.json

The icon command will automatically update your config, but if needed, manually add:

```json
{
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

### 4. Rebuild

```bash
cargo tauri dev  # or cargo tauri build
```

---

## Manual Setup (Advanced)

If you want to create icons manually:

### macOS (.icns)

```bash
# Create iconset directory
mkdir icon.iconset

# Generate different sizes
sips -z 16 16     icon.png --out icon.iconset/icon_16x16.png
sips -z 32 32     icon.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32     icon.png --out icon.iconset/icon_32x32.png
sips -z 64 64     icon.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128   icon.png --out icon.iconset/icon_128x128.png
sips -z 256 256   icon.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256   icon.png --out icon.iconset/icon_256x256.png
sips -z 512 512   icon.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512   icon.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon.png --out icon.iconset/icon_512x512@2x.png

# Convert to .icns
iconutil -c icns icon.iconset -o src-tauri/icons/icon.icns
```

### Windows (.ico)

Use a tool like [ImageMagick](https://imagemagick.org/):

```bash
convert icon.png -define icon:auto-resize=256,128,96,64,48,32,16 src-tauri/icons/icon.ico
```

Or use online converters like [icoconvert.com](https://icoconvert.com/)

### Linux (.png)

Just use your 512x512 or 1024x1024 PNG:

```bash
cp icon.png src-tauri/icons/icon.png
```

---

## Icon Requirements by Platform

### Desktop
- **macOS**: .icns file (auto-generated)
- **Windows**: .ico file (auto-generated)
- **Linux**: .png file (512x512 or larger)

### Mobile
- **iOS**: Multiple PNG sizes (generated during `cargo tauri ios init`)
- **Android**: Multiple PNG sizes (generated during `cargo tauri android init`)

---

## Design Tips

### For Blockchain Apps:
- 🔗 Use chain/link symbolism
- 🔐 Lock or security icons
- 📦 Block/cube designs
- ⛓️ Connected nodes
- 💎 Gem/crystal (valuable data)

### General Icon Design:
- **Simple**: Should be recognizable at 16x16
- **High contrast**: Works on light and dark backgrounds
- **Unique**: Distinguishable from other apps
- **Transparent background**: PNG with alpha channel
- **No text**: Icons without text scale better
- **Square**: 1:1 aspect ratio

---

## Quick Blockchain Icon Ideas

### Option 1: Simple Blocks
```
█
███  ← Stacked blocks
```

### Option 2: Chain Links
```
○-○-○  ← Connected nodes
```

### Option 3: Cube/3D
```
  ╱◢◣╲
 ╱ ◢◣ ╲  ← 3D block
◣━━━━◣
```

---

## Using Online Tools

### Free Icon Generators:
1. **RealFaviconGenerator** - https://realfavicongenerator.net/
2. **Favicon.io** - https://favicon.io/
3. **Canva** - https://www.canva.com/ (with Pro you can export transparent PNG)

### AI-Generated Icons:
```
Prompt: "A modern, minimalist blockchain icon with connected blocks, 
purple and blue gradient, transparent background, flat design"
```

---

## Example: Create Quick Icon with Emoji

For testing, you can use an emoji as an icon:

```bash
# Install imagemagick (macOS)
brew install imagemagick

# Create icon from emoji
convert -background transparent -fill "#6366f1" -font "Apple Color Emoji" \
  -pointsize 800 label:"⛓️" temp-icon.png

# Generate all sizes
cd src-tauri
cargo tauri icon ../temp-icon.png
```

---

## After Adding Icons

Your app will now show your custom icon:
- macOS: Dock and title bar
- Windows: Taskbar and window
- Linux: Application menu
- iOS: Home screen
- Android: App drawer

Rebuild with `cargo tauri dev` or `cargo tauri build` to see the changes!

---

**Pro Tip**: Design your icon in vector format (SVG) first, then export to high-res PNG. This makes it easy to update and rescale later!

