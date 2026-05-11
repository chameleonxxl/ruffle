import url from "url";
import json5 from "json5";
import CopyPlugin from "copy-webpack-plugin";
import TerserPlugin from "terser-webpack-plugin";

function transformPackage(content) {
    const pkg = json5.parse(content);

    // Note: The npm registry requires the version to monotonically increase.
    pkg.version = process.env.npm_package_version;

    return JSON.stringify(pkg);
}

export default function (_env, _argv) {
    const mode = process.env.NODE_ENV || "production";
    console.log(`Building ${mode}...`);

    return {
        mode,
        // dirplayer-rs fork: prepend a tiny entry that lets the host set
        // webpack's chunk publicPath at runtime. Used by the extension's
        // service worker to point chunk URLs at chrome-extension://.../ruffle/
        // when the bundle is injected via chrome.scripting (where
        // `document.currentScript` is null and webpack's default
        // detection falls back to the page URL).
        entry: ["./js/dirplayer-runtime-public-path.js", "./js/ruffle.js"],
        output: {
            path: url.fileURLToPath(new URL("dist", import.meta.url)),
            // dirplayer-rs fork: rename the loader and chunks so they don't
            // collide with stock Ruffle's `ruffle.js` / `core.ruffle.*.js` if
            // another copy is on the same page (e.g. via a browser extension).
            // The HTML/script-tag references that load this file have been
            // updated to match (see public/index.html, the polyfill, and
            // index-dirplayer.html).
            filename: "dirplayer_ruffle.js",
            publicPath: "",
            chunkFilename: "dirplayer_core.ruffle.[contenthash].js",
            // dirplayer-rs fork: webpack derives runtime globals like
            // `webpackChunk${uniqueName}` from the package name by default,
            // which is `ruffle-selfhosted` — identical to stock Ruffle. On
            // pages where stock Ruffle is already loaded (e.g. the Wayback
            // Machine ships its own at `web-static.archive.org/_static/js/
            // ruffle/`), our chunks .push() into stock Ruffle's chunk queue
            // and dynamic imports (including the WASM core) get routed
            // through its webpack runtime — so our `.load()` hangs forever.
            // Setting a fork-specific uniqueName isolates both runtimes.
            uniqueName: "dirplayer_ruffle_selfhosted",
            clean: true,
        },
        performance: {
            assetFilter: (assetFilename) =>
                !/\.(map|wasm)$/i.test(assetFilename),
        },
        optimization: {
            minimizer: [
                new TerserPlugin({
                    terserOptions: {
                        output: {
                            ascii_only: true,
                        },
                    },
                }),
            ],
        },
        devtool: "source-map",
        plugins: [
            new CopyPlugin({
                patterns: [
                    {
                        from: "npm-package.json5",
                        to: "package.json",
                        transform: transformPackage,
                    },
                    { from: "LICENSE*" },
                    { from: "README.md" },
                ],
            }),
        ],
    };
}
