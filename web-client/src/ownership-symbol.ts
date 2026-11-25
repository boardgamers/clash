export const STANDARD_OWNERSHIP_SYMBOLS = ['●', '▲', '■', '◆', '✚', '✕'] as const;
export type StandardOwnershipSymbol = (typeof STANDARD_OWNERSHIP_SYMBOLS)[number];

export interface OwnershipSymbolCanvas {
  fillStyle: string | CanvasGradient | CanvasPattern;
  strokeStyle: string | CanvasGradient | CanvasPattern;
  lineWidth: number;
  lineCap: CanvasLineCap;
  beginPath(): void;
  closePath(): void;
  arc(x: number, y: number, radius: number, startAngle: number, endAngle: number): void;
  moveTo(x: number, y: number): void;
  lineTo(x: number, y: number): void;
  fill(): void;
  stroke(): void;
  fillRect(x: number, y: number, w: number, h: number): void;
  save(): void;
  restore(): void;
  measureText(text: string): {
    actualBoundingBoxLeft?: number;
    actualBoundingBoxRight?: number;
    actualBoundingBoxAscent?: number;
    actualBoundingBoxDescent?: number;
  };
  fillText(text: string, x: number, y: number): void;
}

export function isStandardOwnershipSymbol(symbol: string): symbol is StandardOwnershipSymbol {
  return (STANDARD_OWNERSHIP_SYMBOLS as readonly string[]).includes(symbol);
}

/**
 * Draw a color-blind ownership symbol dead-center at (center, center) on a 2D canvas.
 * Standard symbols use crisp geometric paths so rendering does not depend on system font metrics;
 * non-standard symbols (e.g. custom BGS playerSymbols) center using actualBoundingBox metrics.
 */
export function drawOwnershipSymbol(
  context: OwnershipSymbolCanvas,
  symbol: string,
  center = 32,
  radius = 19,
): void {
  switch (symbol) {
    case '●': {
      context.beginPath();
      context.arc(center, center, radius * 0.96, 0, Math.PI * 2);
      context.fill();
      return;
    }
    case '▲': {
      context.beginPath();
      context.moveTo(center, center - radius * 0.92);
      context.lineTo(center + radius, center + radius * 0.84);
      context.lineTo(center - radius, center + radius * 0.84);
      context.closePath();
      context.fill();
      return;
    }
    case '■': {
      const half = radius * 0.86;
      context.fillRect(center - half, center - half, half * 2, half * 2);
      return;
    }
    case '◆': {
      const r = radius * 1.08;
      context.beginPath();
      context.moveTo(center, center - r);
      context.lineTo(center + r, center);
      context.lineTo(center, center + r);
      context.lineTo(center - r, center);
      context.closePath();
      context.fill();
      return;
    }
    case '✚': {
      const arm = radius;
      const thick = radius * 0.34;
      context.fillRect(center - arm, center - thick, arm * 2, thick * 2);
      context.fillRect(center - thick, center - arm, thick * 2, arm * 2);
      return;
    }
    case '✕': {
      const arm = radius * 0.78;
      context.save();
      context.strokeStyle = context.fillStyle;
      context.lineWidth = radius * 0.52;
      context.lineCap = 'round';
      context.beginPath();
      context.moveTo(center - arm, center - arm);
      context.lineTo(center + arm, center + arm);
      context.moveTo(center + arm, center - arm);
      context.lineTo(center - arm, center + arm);
      context.stroke();
      context.restore();
      return;
    }
    default: {
      const metrics = context.measureText(symbol);
      const left = metrics.actualBoundingBoxLeft;
      const right = metrics.actualBoundingBoxRight;
      const ascent = metrics.actualBoundingBoxAscent;
      const descent = metrics.actualBoundingBoxDescent;
      const x =
        typeof left === 'number' && Number.isFinite(left) && typeof right === 'number' && Number.isFinite(right)
          ? center + (left - right) / 2
          : center;
      const y =
        typeof ascent === 'number' && Number.isFinite(ascent) && typeof descent === 'number' && Number.isFinite(descent)
          ? center + (ascent - descent) / 2
          : center;
      context.fillText(symbol, x, y);
    }
  }
}
