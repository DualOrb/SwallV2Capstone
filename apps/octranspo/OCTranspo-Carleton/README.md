# OCTranspo Bus App with Tauri

Displays useful bus information to students queried from the OCTranspo public api

Make sure to "npm install" in the project context before working on it.

## Run Development

```npm run tauri dev```

This opens both a forwarded port which you can open in the browser **`http://localhost:1420`** (provided by vite), and an app window to observe changes from

## Build and Install App

```npm run tauri build```

This builds the app in the context of the current directory (Should be in root tauri app)

The built .deb files are stored in ```target/release/bundle/deb```

Use ```dpkg -i oc-transo-carleton_0.0.0_amd64.deb``` to install the package

### Runnin the newly installed application

From anywhere, simple do ```oc-transpo-carleton``` to run the application.

### Debugging

If the application does not compile, try replacing the icons with fresh ones in src-tauri/icons and ensure all are there as specified in ```tauri.conf.json```. Sometimes BitBucket will corrupt images in repositories and causes the build process to silently error.
[Repository for Fresh Icons](https://github.com/jeremychone-channel/rust-tauri-intro/tree/main/src-tauri/icons)

You can also debug by action by running ```npm run tauri build --verbose```