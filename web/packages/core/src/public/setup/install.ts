import { PublicAPI } from "./public-api";

import { internalSourceApi } from "../../internal/internal-source-api";

/**
 * Options to use with this specific installation of Ruffle.
 *
 * This is mostly to provide a way to configure environmental settings, like using
 * `onFirstLoad` to potentially configure webpack prior to loading wasm files.
 */
export interface InstallationOptions {
    /**
     * A callback to be run before the very first time Ruffle is loaded.
     * This may be used to configure a bundler prior to asset loading.
     */
    onFirstLoad?: () => void;
}

/**
 * Install this version of Ruffle into the current page.
 *
 * Multiple (or zero) versions of Ruffle may be installed at the same time,
 * and you should use `window.RufflePlayer.newest()` or similar to access the appropriate
 * installation at time of use.
 *
 * @param sourceName The name of this particular
 * Ruffle source. Common convention is "local" for websites that bundle their own Ruffle,
 * "extension" for browser extensions, and something else for other use cases.
 * Names are unique, and last-installed will replace earlier installations with the same name,
 * regardless of what those installations are or which version they represent.
 * @param options Any options used to configure this specific installation of Ruffle.
 */
export function installRuffle(
    sourceName: string,
    options: InstallationOptions = {},
): void {
    // Namespaced under dirplayer_ so the fork doesn't collide with stock
    // Ruffle when both are loaded on the same page (e.g. via a browser
    // extension). dirplayer-rs's selfhosted bundle is the only consumer of
    // this global; stock Ruffle still owns the original `RufflePlayer` name.
    let publicAPI: PublicAPI;
    if (window.dirplayer_RufflePlayer instanceof PublicAPI) {
        publicAPI = window.dirplayer_RufflePlayer;
    } else {
        publicAPI = new PublicAPI(window.dirplayer_RufflePlayer);
        window.dirplayer_RufflePlayer = publicAPI;
    }

    publicAPI.sources[sourceName] = internalSourceApi;
    internalSourceApi.options = options;

    // dirplayer-rs fork: polyfills default to *off*. Upstream Ruffle defaults
    // them on so SWFObject-style detection (`navigator.plugins["Shockwave
    // Flash"]`) and bare `<object>`/`<embed>` tags get auto-handled. In our
    // fork that's exactly what we DON'T want — we're a private Flash backend
    // for dirplayer-rs (which creates players explicitly via
    // `ruffle.createPlayer()` and `.ruffle().load()`) and the page may also
    // have stock Ruffle on it (e.g. via a browser extension) that should win
    // any auto-detection. Pages that genuinely want our fork to polyfill can
    // still opt in with `dirplayer_RufflePlayer.config.polyfills = true`.
    const polyfills =
        "polyfills" in publicAPI.config ? publicAPI.config.polyfills : false;
    if (polyfills !== false) {
        internalSourceApi.pluginPolyfill();
    }
}
