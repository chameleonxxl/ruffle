/**
 * Conditional ruffle loader
 */

import {
    bulkMemory,
    simd,
    saturatedFloatToInt,
    signExtensions,
    referenceTypes,
} from "wasm-feature-detect";
import type { RuffleInstanceBuilder, ZipWriter } from "../dist/ruffle_web";
import { setPolyfillsOnLoad } from "./js-polyfills";

import { internalSourceApi } from "./internal/internal-source-api";

type ProgressCallback = (bytesLoaded: number, bytesTotal: number) => void;

/**
 * Load ruffle from an automatically-detected location.
 *
 * This function returns a new instance of Ruffle and downloads it every time.
 * You should not use it directly; this module will memoize the resource
 * download.
 *
 * @param progressCallback The callback that will be run with Ruffle's download progress.
 * @returns A ruffle-builder constructor that may be used to create new RuffleInstanceBuilder
 * instances.
 */
async function fetchRuffle(
    progressCallback?: ProgressCallback,
): Promise<[typeof RuffleInstanceBuilder, typeof ZipWriter]> {
    // Apply some pure JavaScript polyfills to prevent conflicts with external
    // libraries, if needed.
    setPolyfillsOnLoad();

    // NOTE: Keep this list in sync with $RUSTFLAGS in the CI build config!
    const extensionsSupported: boolean = (
        await Promise.all([
            bulkMemory(),
            simd(),
            saturatedFloatToInt(),
            signExtensions(),
            referenceTypes(),
        ])
    ).every(Boolean);

    // @ts-expect-error TS2367 %FALLBACK_WASM% gets replaced in set_version.ts.
    // %FALLBACK_WASM% is "ruffle_web-wasm_mvp" if this is a dual-wasm build.
    // We don't say we're falling back if we have only an extension build.
    if (!extensionsSupported && "%FALLBACK_WASM%" === "ruffle_web-wasm_mvp") {
        console.log(
            "Some WebAssembly extensions are NOT available, falling back to the vanilla WebAssembly module",
        );
    }

    // Easy "on first load": just set it to something else after the call.
    internalSourceApi.options.onFirstLoad?.();
    internalSourceApi.options.onFirstLoad = () => {};

    // Note: The argument passed to import() has to be a simple string literal,
    // otherwise some bundler will get confused and won't include the module?
    //
    // dirplayer-rs fork: `webpackMode: "eager"` inlines the chunk into the
    // main bundle instead of producing a separate `dirplayer_core.ruffle.
    // {hash}.js`. Pages that monkey-patch HTMLScriptElement.src in the main
    // world (notably the Wayback Machine's `wombat.js`, which rewrites every
    // URL through its playback proxy) silently break webpack's jsonp chunk
    // loader — the chunk request 404s with no error surfaced, and our
    // `bridgeCallMethod(..., "load", ...)` then hangs forever waiting for
    // the WASM init that never starts. Inlining bypasses the script-tag
    // path entirely; the bundle gets bigger but loads reliably.
    const {
        default: init,
        RuffleInstanceBuilder,
        ZipWriter,
    } = await (extensionsSupported
        ? import(/* webpackMode: "eager" */ "../dist/ruffle_web")
        : // @ts-expect-error TS2307 TypeScript compiler is trying to do the import.
          import(/* webpackMode: "eager" */ "../dist/%FALLBACK_WASM%"));
    let response;
    const wasmUrl = extensionsSupported
        ? new URL("../dist/ruffle_web_bg.wasm", import.meta.url)
        : new URL("../dist/%FALLBACK_WASM%_bg.wasm", import.meta.url);
    // dirplayer-rs fork: prefer the pre-wombat `fetch` snapshot taken by
    // `dirplayer-runtime-public-path.js` (first webpack entry). On the
    // Wayback Machine, wombat.js wraps window.fetch and rewrites every URL
    // through its playback proxy — including our `chrome-extension://`
    // WASM URL, which then 404s. The snapshot was captured before wombat
    // had a chance to overwrite `fetch`, so it goes straight to the
    // browser's real fetch.
    const rawFetch = (typeof window !== "undefined"
        && (window as any).__dirplayerOrigFetch) || fetch;
    const wasmResponse = await rawFetch(wasmUrl);
    // The Pale Moon browser lacks full support for ReadableStream.
    // However, ReadableStream itself is defined.
    const readableStreamProperlyDefined =
        typeof ReadableStreamDefaultController === "function";
    if (progressCallback && readableStreamProperlyDefined) {
        const contentLength =
            wasmResponse?.headers?.get("content-length") || "";
        let bytesLoaded = 0;
        // Use parseInt rather than Number so the empty string is coerced to NaN instead of 0
        const bytesTotal = parseInt(contentLength);
        response = new Response(
            new ReadableStream({
                async start(controller) {
                    const reader = wasmResponse.body?.getReader();
                    if (!reader) {
                        throw "Response had no body";
                    }
                    progressCallback(bytesLoaded, bytesTotal);
                    for (;;) {
                        const { done, value } = await reader.read();
                        if (done) {
                            break;
                        }
                        if (value?.byteLength) {
                            bytesLoaded += value?.byteLength;
                        }
                        controller.enqueue(value);
                        progressCallback(bytesLoaded, bytesTotal);
                    }
                    controller.close();
                },
            }),
            wasmResponse,
        );
    } else {
        response = wasmResponse;
    }

    await init({ module_or_path: response });

    // dirplayer-rs fork: the WASM export is renamed to
    // `dirplayer_ruffleRegisterLingoCallback` (see ruffle/web/src/lib.rs and
    // the matching extern import in vm-rust/src/.../sprite.rs) so the fork
    // doesn't collide with stock Ruffle on the same page. Import under the
    // new name and expose it on window so dirplayer-rs's WASM (which calls
    // `js_sys::Reflect::get(&window, "dirplayer_ruffleRegisterLingoCallback")`)
    // can find it.
    const { dirplayer_ruffleRegisterLingoCallback } = await (extensionsSupported
        ? import(/* webpackMode: "eager" */ "../dist/ruffle_web")
        : // @ts-expect-error TS2307 TypeScript compiler is trying to do the import.
          import(/* webpackMode: "eager" */ "../dist/%FALLBACK_WASM%"));

    (window as any).dirplayer_ruffleRegisterLingoCallback = dirplayer_ruffleRegisterLingoCallback;
    console.log("✅ dirplayer_ruffleRegisterLingoCallback exposed globally.");

    return [RuffleInstanceBuilder, ZipWriter];
}

let nativeConstructors: Promise<
    [typeof RuffleInstanceBuilder, typeof ZipWriter]
> | null = null;

/**
 * Obtain an instance of `Ruffle`.
 *
 * This function returns a promise which yields a new `RuffleInstanceBuilder` asynchronously.
 *
 * @param progressCallback The callback that will be run with Ruffle's download progress.
 * @returns A ruffle instance builder.
 */
export async function createRuffleBuilder(
    progressCallback?: ProgressCallback,
): Promise<[RuffleInstanceBuilder, () => ZipWriter]> {
    if (nativeConstructors === null) {
        nativeConstructors = fetchRuffle(progressCallback);
    }

    const constructors = await nativeConstructors;
    return [new constructors[0](), () => new constructors[1]()];
}
