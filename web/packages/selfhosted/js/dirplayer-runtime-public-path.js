// dirplayer-rs fork: set webpack's chunk publicPath at runtime so the
// bundle works when injected via chrome.scripting.executeScript /
// registerContentScripts. In that path `document.currentScript` is null
// (Chrome doesn't create a `<script>` element for its programmatic
// injections), and webpack's default `currentScript.src` lookup falls
// back to the page URL, so chunk loads 404 against the host site.
//
// The extension's service worker sets
// `window.__dirplayerRufflePublicPath` to
// `chrome.runtime.getURL('ruffle/')` BEFORE the Ruffle bundle is
// injected — see extension/src/background.ts. When this entry runs as
// the first webpack module, assigning to `__webpack_public_path__`
// overrides the runtime path used by every subsequent dynamic import.
//
// For the page-loaded polyfill (a regular `<script src=>` tag),
// `document.currentScript` IS valid; the global is undefined; and we
// leave the empty default so webpack's normal currentScript path runs.
// Try the DOM-attribute path first (extension's isolated-world pre-init
// content script stamps `<html data-dirplayer-ruffle-url="...">`),
// then the legacy global (kept for any caller that still uses it).
// Both fall through harmlessly when neither is present, leaving
// webpack's normal currentScript-based detection in place — that
//'s correct for the page-loaded standalone polyfill.
if (typeof document !== "undefined") {
    const fromAttr = document.documentElement
        && document.documentElement.getAttribute("data-dirplayer-ruffle-url");
    const fromGlobal = typeof window !== "undefined"
        ? window.__dirplayerRufflePublicPath
        : undefined;
    const url = fromAttr || fromGlobal;
    if (url) {
        // eslint-disable-next-line no-undef
        __webpack_public_path__ = url;
    }
}
