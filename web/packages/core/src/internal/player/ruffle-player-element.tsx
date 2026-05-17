import type { DataLoadOptions, URLLoadOptions } from "../../public/config";
import { MovieMetadata, PlayerElement, ReadyState } from "../../public/player";
import { InnerPlayer } from "./inner";
import { APIVersions } from "../../public/player";
import { PlayerV1Impl } from "./impl_v1";

/**
 * The ruffle player element that should be inserted onto the page.
 *
 * This element will represent the rendered and intractable flash movie.
 */
export class RufflePlayerElement extends HTMLElement implements PlayerElement {
    #inner: InnerPlayer;
    #legacyFSCommandHandler: ((command: string, args: string) => void) | null =
        null;

    get onFSCommand(): ((command: string, args: string) => void) | null {
        return this.#legacyFSCommandHandler;
    }

    set onFSCommand(value: ((command: string, args: string) => void) | null) {
        this.#legacyFSCommandHandler = value;
    }

    get readyState(): ReadyState {
        return this.#inner._readyState;
    }

    get metadata(): MovieMetadata | null {
        return this.#inner.metadata;
    }

    constructor() {
        super();
        this.#inner = new InnerPlayer(
            this,
            () => this.debugPlayerInfo(),
            (name) => {
                try {
                    Object.defineProperty(this, name, {
                        value: (...args: unknown[]) => {
                            return this.#inner.callExternalInterface(
                                name,
                                args,
                            );
                        },
                        configurable: true,
                    });
                } catch (error) {
                    console.warn(
                        `Error setting ExternalInterface legacy callback for ${name}`,
                        error,
                    );
                }
            },
        );
        this.#inner.addFSCommandHandler((command, args) => {
            this.#legacyFSCommandHandler?.(command, args);
        });
    }

    /**
     * dirplayer fork: forward `dirplayer_addOpenUrlHandler` to the inner
     * player so flashPlayerManager.ts (which only sees the Web Component
     * element returned by `dirplayer_RufflePlayer.newest()`) can register
     * its `event:` URL handler. Stock callers won't see this method —
     * keeps Director's getURL routing isolated to the dirplayer fork.
     */
    dirplayer_addOpenUrlHandler(
        handler: (url: string, target: string) => boolean,
    ): void {
        this.#inner.dirplayer_addOpenUrlHandler(handler);
    }

    /**
     * dirplayer fork: forward `dirplayer_dispatchPointer` to the inner
     * player so flashPlayerManager.ts can inject Director-side clicks
     * straight into Ruffle's input pipeline. See InnerPlayer for why
     * synthesised DOM PointerEvents don't suffice.
     */
    dirplayer_dispatchPointer(
        type: "down" | "up" | "move",
        localX: number,
        localY: number,
    ): boolean {
        return this.#inner.dirplayer_dispatchPointer(type, localX, localY);
    }

    ruffle<V extends keyof APIVersions = 1>(version?: V): APIVersions[V] {
        const v = version ?? 1;
        if (v === 1) {
            return new PlayerV1Impl(this.#inner) as APIVersions[V];
        }
        throw new Error(`Version ${version} not supported.`);
    }

    get loadedConfig(): URLLoadOptions | DataLoadOptions | null {
        return this.#inner.loadedConfig ?? null;
    }

    connectedCallback(): void {
        this.#inner.updateStyles();
    }

    static get observedAttributes(): string[] {
        return ["width", "height", "align"];
    }

    attributeChangedCallback(
        name: string,
        _oldValue: string | undefined,
        _newValue: string | undefined,
    ): void {
        if (RufflePlayerElement.observedAttributes.includes(name)) {
            this.#inner.updateStyles();
        }
    }

    disconnectedCallback(): void {
        this.#inner.destroy();
    }

    async reload(): Promise<void> {
        await this.#inner.reload();
    }

    async load(
        options: string | URLLoadOptions | DataLoadOptions,
        isPolyfillElement: boolean = false,
    ): Promise<void> {
        await this.#inner.load(options, isPolyfillElement);
    }

    play(): void {
        this.#inner.play();
    }

    get isPlaying(): boolean {
        return this.#inner.isPlaying;
    }

    get volume(): number {
        return this.#inner.volume;
    }

    set volume(value: number) {
        this.#inner.volume = value;
    }

    get fullscreenEnabled(): boolean {
        return this.#inner.fullscreenEnabled;
    }

    get isFullscreen(): boolean {
        return this.#inner.isFullscreen;
    }

    setFullscreen(isFull: boolean): void {
        this.#inner.setFullscreen(isFull);
    }

    enterFullscreen(): void {
        this.#inner.enterFullscreen();
    }

    exitFullscreen(): void {
        this.#inner.exitFullscreen();
    }

    async downloadSwf(): Promise<void> {
        await this.#inner.downloadSwf();
    }

    pause(): void {
        this.#inner.pause();
    }

    set traceObserver(observer: ((message: string) => void) | null) {
        this.#inner.traceObserver = observer;
    }

    protected debugPlayerInfo(): string {
        return "";
    }

    public PercentLoaded(): number {
        // [NA] This is a stub - we need to research how this is actually implemented (is it just base swf loadedBytes?)
        if (this.#inner._readyState === ReadyState.Loaded) {
            return 100;
        } else {
            return 0;
        }
    }

    public GetVariable(path: string): any {
        return this.#inner.getVariable(path);
    }

    public SetVariable(path: string, value: any): boolean {
        return this.#inner.setVariable(path, value);
    }

    public CallFunction(path: string, args: any[] = []): any {
        return this.#inner.callFunction(path, args);
    }

    public GotoFrame(frame: number, stop: boolean = true): void {
        this.#inner.gotoFrame(frame, stop);
    }

    public SetCallback(flashObjectPath: string, eventName: string, callbackId: string): boolean {
        return this.#inner.setCallback(flashObjectPath, eventName, callbackId);
    }

    get config(): URLLoadOptions | DataLoadOptions | object {
        return this.#inner.config;
    }

    set config(value: URLLoadOptions | DataLoadOptions | object) {
        this.#inner.config = value;
    }

    displayMessage(message: string): void {
        this.#inner.displayMessage(message);
    }
}

/**
 * Copies attributes and children from another element to a target element.
 * Used by the polyfill elements, RuffleObject and RuffleEmbed.
 *
 * @param element The element to copy all attributes from.
 * @param destination The element to copy all attributes to.
 */
export function copyElement(element: Element, destination: Element): void {
    if (element) {
        for (const attribute of element.attributes) {
            if (attribute.specified) {
                // Issue 468: Chrome "Click to Active Flash" box stomps on title attribute
                if (
                    attribute.name === "title" &&
                    attribute.value === "Adobe Flash Player"
                ) {
                    continue;
                }

                try {
                    destination.setAttribute(attribute.name, attribute.value);
                } catch (_err) {
                    // The embed may have invalid attributes, so handle these gracefully.
                    console.warn(
                        `Unable to set attribute ${attribute.name} on Ruffle instance`,
                    );
                }
            }
        }

        for (const node of Array.from(element.children)) {
            destination.appendChild(node);
        }
    }
}
