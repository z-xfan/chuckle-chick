import type { LoadedPetAsset, PetAnimation, PetAtlas, PetManifest } from "./types";

type JsonFetcher = (url: string) => Promise<unknown>;

const defaultJsonFetcher: JsonFetcher = async (url) => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`资源读取失败：${url} (${response.status})`);
  return response.json() as Promise<unknown>;
};

export async function loadPetAsset(
  baseUrl: string,
  fetchJson: JsonFetcher = defaultJsonFetcher,
): Promise<LoadedPetAsset> {
  const normalizedBaseUrl = baseUrl.replace(/\/$/, "");
  const [manifestValue, atlasValue] = await Promise.all([
    fetchJson(`${normalizedBaseUrl}/pet.json`),
    fetchJson(`${normalizedBaseUrl}/atlas.json`),
  ]);
  const manifest = parsePetManifest(manifestValue);
  const atlas = parsePetAtlas(atlasValue);

  if (manifest.spritesheetPath !== atlas.image) {
    throw new Error("pet.json 与 atlas.json 指向了不同的图集文件");
  }

  return {
    baseUrl: normalizedBaseUrl,
    manifest,
    atlas,
    spritesheetUrl: `${normalizedBaseUrl}/${manifest.spritesheetPath}`,
  };
}

export function parsePetManifest(value: unknown): PetManifest {
  const record = requireRecord(value, "pet.json");
  if (record.spriteVersionNumber !== 2) throw new Error("只支持 spriteVersionNumber 2");

  return {
    id: requireString(record.id, "pet.id"),
    displayName: requireString(record.displayName, "pet.displayName"),
    description: requireString(record.description, "pet.description"),
    spriteVersionNumber: 2,
    spritesheetPath: requireString(record.spritesheetPath, "pet.spritesheetPath"),
  };
}

export function parsePetAtlas(value: unknown): PetAtlas {
  const record = requireRecord(value, "atlas.json");
  if (record.version !== 2) throw new Error("只支持 atlas version 2");

  const canvas = requireRecord(record.canvas, "atlas.canvas");
  const grid = requireRecord(record.grid, "atlas.grid");
  const animationsRecord = requireRecord(record.animations, "atlas.animations");
  const animations = Object.fromEntries(
    Object.entries(animationsRecord).map(([name, animation]) => [name, parseAnimation(name, animation)]),
  );
  if (!animations.idle) throw new Error("atlas.animations 必须包含 idle");

  const parsed: PetAtlas = {
    version: 2,
    image: requireString(record.image, "atlas.image"),
    canvas: {
      width: requirePositiveInteger(canvas.width, "atlas.canvas.width"),
      height: requirePositiveInteger(canvas.height, "atlas.canvas.height"),
    },
    grid: {
      columns: requirePositiveInteger(grid.columns, "atlas.grid.columns"),
      rows: requirePositiveInteger(grid.rows, "atlas.grid.rows"),
      cellWidth: requirePositiveInteger(grid.cellWidth, "atlas.grid.cellWidth"),
      cellHeight: requirePositiveInteger(grid.cellHeight, "atlas.grid.cellHeight"),
    },
    animations,
    lookDirections: parseLookDirections(record.lookDirections),
  };

  if (parsed.grid.columns * parsed.grid.cellWidth !== parsed.canvas.width) {
    throw new Error("图集宽度与网格列定义不一致");
  }
  if (parsed.grid.rows * parsed.grid.cellHeight !== parsed.canvas.height) {
    throw new Error("图集高度与网格行定义不一致");
  }
  for (const [name, animation] of Object.entries(parsed.animations)) {
    const columns =
      animation.frameColumns ??
      Array.from({ length: animation.frames }, (_, column) => column);
    if (
      animation.row >= parsed.grid.rows ||
      columns.some((column) => column >= parsed.grid.columns)
    ) {
      throw new Error(`动画 ${name} 超出图集网格范围`);
    }
  }

  return parsed;
}

function parseAnimation(name: string, value: unknown): PetAnimation {
  const record = requireRecord(value, `animation.${name}`);
  const frames = requirePositiveInteger(record.frames, `animation.${name}.frames`);
  if (!Array.isArray(record.durationsMs) || record.durationsMs.length !== frames) {
    throw new Error(`动画 ${name} 的帧数与 durationsMs 数量不一致`);
  }
  const frameColumns =
    record.frameColumns === undefined
      ? undefined
      : parseFrameColumns(name, record.frameColumns, frames);
  return {
    row: requireNonNegativeInteger(record.row, `animation.${name}.row`),
    frames,
    durationsMs: record.durationsMs.map((duration, index) =>
      requirePositiveInteger(duration, `animation.${name}.durationsMs[${index}]`),
    ),
    frameColumns,
  };
}

function parseFrameColumns(name: string, value: unknown, frames: number): number[] {
  if (!Array.isArray(value) || value.length !== frames) {
    throw new Error(`动画 ${name} 的帧数与 frameColumns 数量不一致`);
  }
  return value.map((column, index) =>
    requireNonNegativeInteger(column, `animation.${name}.frameColumns[${index}]`),
  );
}

function parseLookDirections(value: unknown): PetAtlas["lookDirections"] {
  if (!Array.isArray(value)) throw new Error("atlas.lookDirections 必须是数组");
  return value.map((direction, index) => {
    const record = requireRecord(direction, `lookDirections[${index}]`);
    return {
      degrees: requireNumber(record.degrees, `lookDirections[${index}].degrees`),
      label: requireString(record.label, `lookDirections[${index}].label`),
      row: requireNonNegativeInteger(record.row, `lookDirections[${index}].row`),
      column: requireNonNegativeInteger(record.column, `lookDirections[${index}].column`),
    };
  });
}

function requireRecord(value: unknown, path: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${path} 必须是对象`);
  }
  return value as Record<string, unknown>;
}

function requireString(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${path} 必须是非空字符串`);
  return value;
}

function requireNumber(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${path} 必须是有限数字`);
  return value;
}

function requirePositiveInteger(value: unknown, path: string): number {
  const number = requireNumber(value, path);
  if (!Number.isInteger(number) || number <= 0) throw new Error(`${path} 必须是正整数`);
  return number;
}

function requireNonNegativeInteger(value: unknown, path: string): number {
  const number = requireNumber(value, path);
  if (!Number.isInteger(number) || number < 0) throw new Error(`${path} 必须是非负整数`);
  return number;
}
