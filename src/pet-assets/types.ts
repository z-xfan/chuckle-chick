export interface PetManifest {
  id: string;
  displayName: string;
  description: string;
  spriteVersionNumber: 2;
  spritesheetPath: string;
}

export interface PetAnimation {
  row: number;
  frames: number;
  durationsMs: number[];
}

export interface PetAtlas {
  version: 2;
  image: string;
  canvas: { width: number; height: number };
  grid: {
    columns: number;
    rows: number;
    cellWidth: number;
    cellHeight: number;
  };
  animations: Record<string, PetAnimation>;
  lookDirections: Array<{
    degrees: number;
    label: string;
    row: number;
    column: number;
  }>;
}

export interface LoadedPetAsset {
  baseUrl: string;
  manifest: PetManifest;
  atlas: PetAtlas;
  spritesheetUrl: string;
}
