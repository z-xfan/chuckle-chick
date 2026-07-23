import { AnimationTimeline } from "@/pet-core/AnimationTimeline";
import { resolveAnimationFrameColumns } from "@/pet-core/AnimationFrameSequence";
import type { LoadedPetAsset } from "@/pet-assets/types";

interface SpriteFrame {
  row: number;
  column: number;
}

export class PixiPetRenderer {
  private readonly canvas = document.createElement("canvas");
  private readonly context = this.canvas.getContext("2d", { alpha: true });
  private asset?: LoadedPetAsset;
  private spritesheet?: HTMLImageElement;
  private frames: SpriteFrame[] = [];
  private timeline?: AnimationTimeline;
  private currentFrame = -1;
  private resizeObserver?: ResizeObserver;
  private animationFrameId?: number;
  private lastTickMs?: number;

  constructor(private readonly host: HTMLElement) {
    if (!this.context) throw new Error("当前环境不支持 Canvas 2D 渲染");
  }

  async initialize(asset: LoadedPetAsset): Promise<void> {
    this.asset = asset;
    this.spritesheet = await loadImage(asset.spritesheetUrl);
    this.canvas.setAttribute("aria-hidden", "true");
    this.host.appendChild(this.canvas);
    this.resizeObserver = new ResizeObserver(() => this.resizeToHost());
    this.resizeObserver.observe(this.host);
    this.resizeToHost();
    this.startTicker();
  }

  async play(animationName: string): Promise<void> {
    if (!this.asset) throw new Error("PixiPetRenderer 尚未初始化");
    const animation = this.asset.atlas.animations[animationName];
    if (!animation) throw new Error(`未找到动画：${animationName}`);

    this.frames = resolveAnimationFrameColumns(animation).map((column) => ({
      row: animation.row,
      column,
    }));
    this.timeline = new AnimationTimeline(animation);
    this.currentFrame = 0;
    this.drawCurrentFrame();
  }

  async showLookDirection(row: number, column: number): Promise<void> {
    if (!this.asset) throw new Error("PixiPetRenderer 尚未初始化");
    this.frames = [{ row, column }];
    this.timeline = undefined;
    this.currentFrame = 0;
    this.drawCurrentFrame();
  }

  destroy(): void {
    if (this.animationFrameId !== undefined) window.cancelAnimationFrame(this.animationFrameId);
    this.animationFrameId = undefined;
    this.resizeObserver?.disconnect();
    this.canvas.remove();
    this.frames = [];
  }

  private startTicker(): void {
    const tick = (timestamp: number): void => {
      const deltaMs = this.lastTickMs === undefined ? 0 : timestamp - this.lastTickMs;
      this.lastTickMs = timestamp;
      this.advance(deltaMs);
      this.animationFrameId = window.requestAnimationFrame(tick);
    };
    this.animationFrameId = window.requestAnimationFrame(tick);
  }

  private advance(deltaMs: number): void {
    if (!this.timeline) return;
    const nextFrame = this.timeline.advance(deltaMs);
    if (nextFrame === this.currentFrame) return;
    this.currentFrame = nextFrame;
    this.drawCurrentFrame();
  }

  private resizeToHost(): void {
    const width = this.host.clientWidth;
    const height = this.host.clientHeight;
    if (width <= 0 || height <= 0) return;

    const ratio = Math.max(window.devicePixelRatio || 1, 1);
    this.canvas.width = Math.round(width * ratio);
    this.canvas.height = Math.round(height * ratio);
    this.canvas.style.width = `${width}px`;
    this.canvas.style.height = `${height}px`;
    this.context?.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.drawCurrentFrame();
  }

  private drawCurrentFrame(): void {
    if (!this.asset || !this.spritesheet || !this.context) return;
    const frame = this.frames[this.currentFrame];
    if (!frame) return;

    const width = this.host.clientWidth;
    const height = this.host.clientHeight;
    if (width <= 0 || height <= 0) return;

    const { cellWidth, cellHeight } = this.asset.atlas.grid;
    this.context.clearRect(0, 0, width, height);
    this.context.drawImage(
      this.spritesheet,
      frame.column * cellWidth,
      frame.row * cellHeight,
      cellWidth,
      cellHeight,
      0,
      0,
      width,
      height,
    );
  }
}

async function loadImage(url: string): Promise<HTMLImageElement> {
  const image = new Image();
  image.decoding = "async";
  image.src = url;
  if (!image.complete) {
    await new Promise<void>((resolve, reject) => {
      image.addEventListener("load", () => resolve(), { once: true });
      image.addEventListener("error", () => reject(new Error(`图片资源读取失败：${url}`)), {
        once: true,
      });
    });
  }
  await image.decode?.().catch(() => undefined);
  return image;
}
