import * as THREE from 'three';
import { mount, unmount, flushSync } from 'svelte';
import { civilizationIcon, civilizationImages } from './civilization-emblems';

const images = new Map<string, Promise<HTMLImageElement>>();
function emblemImage(civilization: string) {
  const cached = images.get(civilization);
  if (cached) return cached;
  const pending = Promise.resolve().then(async () => {
    let source = civilizationImages[civilization];
    if (!source) {
      // Render the same icon as the UI so map and player emblems stay in sync.
      const target = document.createElement('div');
      const icon = mount(civilizationIcon(civilization), {
        target,
        props: { size: 128, color: '#000', strokeWidth: 2.5 },
      });
      flushSync();
      const svg = new XMLSerializer().serializeToString(target.querySelector('svg')!);
      void unmount(icon);
      source = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    }
    const image = new Image();
    image.src = source;
    await image.decode();
    return image;
  });
  images.set(civilization, pending);
  return pending;
}

/** One texture per civilization/color, shared by all its cities across board rebuilds. */
export class CivilizationFlags {
  private textures = new Map<string, THREE.CanvasTexture>();
  private disposed = false;
  constructor(private invalidate: () => void) {}

  texture(civilization: string, color: string) {
    const key = JSON.stringify([civilization, color]);
    const cached = this.textures.get(key);
    if (cached) return cached;
    const canvas = document.createElement('canvas');
    canvas.width = 192;
    canvas.height = 128;
    const context = canvas.getContext('2d')!;
    context.fillStyle = color;
    context.fillRect(0, 0, canvas.width, canvas.height);
    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    this.textures.set(key, texture);
    void emblemImage(civilization)
      .then((image) => {
        if (this.disposed) return;
        const emblem = document.createElement('canvas');
        emblem.width = emblem.height = 104;
        const ink = emblem.getContext('2d')!;
        const scale = 104 / Math.max(image.naturalWidth, image.naturalHeight);
        const width = image.naturalWidth * scale,
          height = image.naturalHeight * scale;
        ink.drawImage(image, (104 - width) / 2, (104 - height) / 2, width, height);
        ink.globalCompositeOperation = 'source-in';
        const rgb = new THREE.Color(color);
        // Contrast follows custom player colors too, including the light barbarian banner.
        const luminance = 0.2126 * rgb.r + 0.7152 * rgb.g + 0.0722 * rgb.b;
        ink.fillStyle = luminance > 0.32 ? '#293b37' : '#fff4dc';
        ink.fillRect(0, 0, 104, 104);
        context.drawImage(emblem, 44, 12);
        texture.needsUpdate = true;
        this.invalidate();
      })
      .catch(() => {
        // Keep the ownership color if an emblem asset cannot be decoded.
      });
    return texture;
  }

  dispose() {
    this.disposed = true;
    for (const texture of this.textures.values()) texture.dispose();
    this.textures.clear();
  }
}
