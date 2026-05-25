<div align="right">

English | [中文](./README.md)

</div>

<div align="center">

<img src="copy-creator/public/logo.png" alt="Copy Creator Logo" width="120">

# Copy Creator

**Desktop Productivity Tool**

Clipboard Manager · Quick Phrases · Translation · Quick Access

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS-brightgreen.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-ffc131.svg)
![React](https://img.shields.io/badge/React-19-61dafb.svg)

</div>

---

## Overview

Copy Creator is a lightweight desktop productivity tool (supporting Windows and macOS) that appears as a floating window and minimizes to the system tray when closed. It integrates clipboard history management, quick phrases, translation, and quick access features, helping users improve text processing efficiency in their daily work.

## Features

### 📋 Clipboard Manager
- Automatically records text, image, link, and file path copy history
- Auto-detects and categorizes links (http/https/ftp/ftps)
- Auto-imports image files (JPG/PNG, under 3MB)
- Image deduplication and automatic thumbnail generation
- Keyword search for quick access to historical content
- Filter by type: All / Text / Image / Link / File
- One-click paste to the current cursor position
- Configurable retention period (1 week / 1 month / 3 months) with automatic cleanup

### ⚡ Quick Phrases
- Organize common phrases and code snippets by scenario groups
- Customizable groups for flexible content organization
- Click to paste directly without manual copying

### 🌐 Translation
- **AI Translation**: Compatible with OpenAI API format, customizable endpoint, API key, and model
- **Google Translate**: Free mode (no API key required) or paid mode, with proxy support
- Local caching of translation results to avoid redundant requests
- Supports 11 target languages: Chinese, English, Japanese, Korean, French, German, Spanish, Russian, Arabic, Thai, Vietnamese
- Auto-detects source language

### 🎯 Quick Access
- **Radial Menu**: Mouse gesture (Ctrl+Alt+RightClick, Windows only) or keyboard shortcut triggers a floating overlay at cursor position with Clipboard and Phrases tabs; hover to select, release to paste
- **Quick Translate Popup**: Keyboard shortcut triggers automatic clipboard text translation to Chinese, showing results in a floating popup at cursor position

### ⚙️ System Features
- Global hotkey to show/hide window (customizable)
- Window always-on-top display
- Light/Dark theme switching
- Launch at system startup (minimizes to tray on auto-launch)
- Custom data storage path (with migration support)
- Resizable sidebar (60-130px, auto-collapses when narrow)
- macOS vibrancy frosted glass / Windows Mica/Acrylic effects
- System tray with localized menu that updates dynamically with language settings

## Tech Stack

| Layer | Technology |
|:---:|:---|
| Desktop Framework | [Tauri 2.x](https://tauri.app/) (Rust) |
| Frontend Framework | React 19 + TypeScript |
| Build Tool | [Vite](https://vitejs.dev/) |
| UI Styling | Pure CSS (iOS-style frosted glass effect) |
| State Management | [Zustand](https://zustand-demo.pmnd.rs/) |
| Local Storage | SQLite (rusqlite, bundled) |
| Internationalization | react-i18next (Simplified Chinese / English) |
| Keyboard Simulation | enigo 0.3 |
| HTTP Client | reqwest 0.12 |
| Image Processing | image 0.25 |

## Download

Go to the [Releases](https://github.com/hu-qi-jia/copy-creator/releases) page to download the latest installer:

| Package | Description |
|:---|:---|
| `Copy Creator_x64-setup.exe` | NSIS Installer |
| `Copy Creator_x64_zh-CN.msi` | MSI Installer (Chinese) |

**System Requirements**: Windows 11 / macOS

## Usage Guide

### Getting Started

1. **Launch the App**: Double-click the desktop icon after installation, the app will appear as a floating window
2. **System Tray**: When you close the window, the app automatically minimizes to the system tray and continues running in the background
3. **Show Window**: Use the global hotkey (configurable in settings) to quickly show/hide the window

### Clipboard Feature

1. Copy any text or image, and the system will automatically record it to clipboard history
2. Click the tray icon or use the hotkey to open the main window
3. Switch to the "Clipboard" tab to browse or search history
4. Click any record to paste it directly to the current cursor position

### Quick Phrases Feature

1. Switch to the "Phrases" tab
2. Click "New Group" to create scenario groups (e.g., customer service scripts, code snippets)
3. Add commonly used phrases to the group
4. When needed, click a phrase to paste it to the current input position

### Translation Feature

1. Switch to the "Translation" tab
2. Enter or paste the text to translate
3. Select translation direction (e.g., Chinese → English)
4. Click the translate button to get results
5. For AI translation, please configure the API endpoint and key in settings

### Quick Access

#### Radial Menu
1. Enable the radial menu in settings
2. Trigger with mouse gesture (Ctrl+Alt+RightClick, Windows only) or custom keyboard shortcut
3. In the floating overlay, hover to select clipboard records or phrases
4. Release mouse button or click to paste the selected content

#### Quick Translate Popup
1. Configure the quick translate keyboard shortcut in settings
2. Copy the text you want to translate
3. Press the shortcut key — the popup automatically reads the clipboard and translates to Chinese
4. Click the translation result to copy it

### Personalization Settings

- **Hotkeys**: Customize global hotkeys, radial menu shortcut, and quick translate shortcut
- **Theme**: Switch between light and dark themes
- **Language**: Switch between Chinese / English UI language
- **Launch at Startup**: Enable or disable auto-start on boot
- **Storage Management**: Custom data storage path, configure clipboard history retention period
- **Translation Engine**: Choose Google Translate or AI translation, configure API parameters

## Development Guide

### Prerequisites

- [Node.js](https://nodejs.org/) (18+ recommended)
- [pnpm](https://pnpm.io/)
- [Rust](https://www.rust-lang.org/)
- [Tauri CLI](https://tauri.app/)

### Local Development

```bash
# Clone the repository
git clone https://github.com/hu-qi-jia/copy-creator.git
cd copy-creator/copy-creator

# Install dependencies
pnpm install

# Start development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

## Project Structure

```
copy-creator/
├── src/                    # Frontend source code
│   ├── components/         # React components
│   │   ├── RadialMenu/     # Radial menu floating overlay
│   │   ├── TranslatePopup/ # Quick translate popup
│   │   ├── settings/       # Settings panel sections
│   │   └── ...
│   ├── pages/              # Page components
│   │   ├── ClipboardPage/  # Clipboard manager
│   │   ├── PhrasePage/     # Quick phrases
│   │   └── TranslationPage # Translation
│   ├── stores/             # Zustand state management
│   ├── styles/             # CSS style files
│   ├── i18n/               # Internationalization config
│   └── types/              # TypeScript type definitions
├── src-tauri/              # Tauri backend source code
│   ├── src/                # Rust source code
│   │   ├── clipboard.rs    # Clipboard monitoring & recording
│   │   ├── paste.rs        # Paste operations
│   │   ├── translator.rs   # Translation engines
│   │   ├── shortcut.rs     # Shortcuts & gestures
│   │   ├── tray.rs         # System tray
│   │   ├── db.rs           # SQLite database
│   │   └── lib.rs          # Main entry point
│   └── Cargo.toml          # Rust dependency config
├── public/                 # Static assets
└── package.json            # Frontend dependency config
```

## License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">

If you find this project helpful, feel free to give it a Star!

</div>
