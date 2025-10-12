# Setup Instructions

## Icon Setup (Required for Windows Build)

The project needs a proper application icon to build on Windows. Follow these steps:

### Option 1: Generate from a PNG (Recommended)

1. Create or download a 1024x1024 PNG icon named `app-icon.png` in the `apps/desktop/` directory
2. Run the icon generator:
   ```bash
   cd apps/desktop
   npx @tauri-apps/cli icon
   ```

This will generate all required icon formats including `.ico` for Windows.

### Option 2: Use a Placeholder

For development purposes, you can create a simple placeholder:

```bash
cd apps/desktop
# Create a simple SVG that can be used as placeholder
cat > app-icon.svg << 'EOF'
<svg width="1024" height="1024" xmlns="http://www.w3.org/2000/svg">
  <rect width="1024" height="1024" fill="#2c3e50"/>
  <text x="512" y="512" font-size="200" text-anchor="middle" dominant-baseline="middle" fill="white">MS</text>
</svg>
EOF

# Convert to PNG (requires ImageMagick or similar)
# If you don't have ImageMagick, just download any 1024x1024 PNG icon
convert app-icon.svg -resize 1024x1024 app-icon.png

# Generate icons
npx @tauri-apps/cli icon
```

## After Icon Setup

Once icons are generated, you can build the project:

```bash
cd apps/desktop
npm run tauri dev
```

## Troubleshooting

If you encounter "icon.ico not found" errors:
1. Make sure `src-tauri/icons/icon.ico` exists
2. Re-run the icon generator: `npx @tauri-apps/cli icon`
3. If all else fails, download a valid .ico file and place it in `src-tauri/icons/`
