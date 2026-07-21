import { Application, Assets, Rectangle, Sprite, Texture, type Ticker } from "pixi.js";

import { AnimationTimeline } from "@/pet-core/AnimationTimeline";
import type { LoadedPetAsset } from "@/pet-assets/types";

export class PixiPetRenderer {
  private readonly app = new Application();
  private asset?: LoadedPetAsset;
  private frames: Texture[] = [];
  private sprite?: Sprite;
  private timeline?: AnimationTimeline;
  private currentFrame = -1;
  private resizeObserver?: ResizeObserver;

  constructor(private readonly host: HTMLElement) {}

  async initialize(asset: LoadedPetAsset): Promise<void> {
    this.asset = asset;
    await this.app.init({
      resizeTo: this.host,
      backgroundAlpha: 0,
      antialias: true,
      autoDensity: true,
      resolution: window.devicePixelRatio,
    });
    this.host.appendChild(this.app.canvas);
    this.app.ticker.add(this.onTick);
    this.resizeObserver = new ResizeObserver(() => this.resizeToHost());
    this.resizeObserver.observe(this.host);
  }

  async play(animationName: string): Promise<void> {
    if (!this.asset) throw new Error("PixiPetRenderer 尚未初始化");
    const animation = this.asset.atlas.animations[animationName];
    if (!animation) throw new Error(`未找到动画：${animationName}`);

    const sheet = await Assets.load<Texture>(this.asset.spritesheetUrl);
    const frames = Array.from({ length: animation.frames }, (_, column) =>
      new Texture({
        source: sheet.source,
        frame: new Rectangle(
          column * this.asset!.atlas.grid.cellWidth,
          animation.row * this.asset!.atlas.grid.cellHeight,
          this.asset!.atlas.grid.cellWidth,
          this.asset!.atlas.grid.cellHeight,
        ),
      }),
    );
    this.replaceFrames(frames);
    this.timeline = new AnimationTimeline(animation);
    this.currentFrame = 0;
  }

  async showLookDirection(row: number, column: number): Promise<void> {
    if (!this.asset) throw new Error("PixiPetRenderer 尚未初始化");
    const sheet = await Assets.load<Texture>(this.asset.spritesheetUrl);
    const { cellWidth, cellHeight } = this.asset.atlas.grid;
    this.replaceFrames([
      new Texture({
        source: sheet.source,
        frame: new Rectangle(column * cellWidth, row * cellHeight, cellWidth, cellHeight),
      }),
    ]);
    this.timeline = undefined;
    this.currentFrame = 0;
  }

  destroy(): void {
    this.app.ticker.remove(this.onTick);
    this.resizeObserver?.disconnect();
    this.sprite?.destroy();
    this.sprite = undefined;
    this.frames.forEach((frame) => frame.destroy(false));
    this.frames = [];
    this.app.destroy(true, { children: true });
  }

  private readonly onTick = (ticker: Ticker): void => {
    if (!this.timeline || !this.sprite) return;
    const nextFrame = this.timeline.advance(ticker.deltaMS);
    if (nextFrame === this.currentFrame) return;

    const texture = this.frames[nextFrame];
    if (!texture) return;
    this.sprite.texture = texture;
    this.currentFrame = nextFrame;
  };

  private replaceFrames(frames: Texture[]): void {
    this.sprite?.destroy();
    this.frames.forEach((frame) => frame.destroy(false));
    this.frames = frames;
    this.sprite = new Sprite(frames[0]);
    this.app.stage.addChild(this.sprite);
    this.resizeToHost();
  }

  private resizeToHost(): void {
    const width = this.host.clientWidth;
    const height = this.host.clientHeight;
    if (width <= 0 || height <= 0) return;
    this.app.renderer.resize(width, height);
    if (this.sprite) {
      this.sprite.width = width;
      this.sprite.height = height;
    }
  }
}
