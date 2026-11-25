import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  drawOwnershipSymbol,
  isStandardOwnershipSymbol,
  STANDARD_OWNERSHIP_SYMBOLS,
  type OwnershipSymbolCanvas,
} from './ownership-symbol.ts';

function mockCanvas(metrics: {
  actualBoundingBoxLeft?: number;
  actualBoundingBoxRight?: number;
  actualBoundingBoxAscent?: number;
  actualBoundingBoxDescent?: number;
} = {}) {
  const calls: Array<{ op: string; args: unknown[] }> = [];
  const canvas: OwnershipSymbolCanvas = {
    fillStyle: '#25372e',
    strokeStyle: '#2d392f',
    lineWidth: 3,
    lineCap: 'butt',
    beginPath() {
      calls.push({ op: 'beginPath', args: [] });
    },
    closePath() {
      calls.push({ op: 'closePath', args: [] });
    },
    arc(x, y, radius, startAngle, endAngle) {
      calls.push({ op: 'arc', args: [x, y, radius, startAngle, endAngle] });
    },
    moveTo(x, y) {
      calls.push({ op: 'moveTo', args: [x, y] });
    },
    lineTo(x, y) {
      calls.push({ op: 'lineTo', args: [x, y] });
    },
    fill() {
      calls.push({ op: 'fill', args: [] });
    },
    stroke() {
      calls.push({ op: 'stroke', args: [] });
    },
    fillRect(x, y, w, h) {
      calls.push({ op: 'fillRect', args: [x, y, w, h] });
    },
    save() {
      calls.push({ op: 'save', args: [] });
    },
    restore() {
      calls.push({ op: 'restore', args: [] });
    },
    measureText(text) {
      calls.push({ op: 'measureText', args: [text] });
      return metrics;
    },
    fillText(text, x, y) {
      calls.push({ op: 'fillText', args: [text, x, y] });
    },
  };
  return { canvas, calls };
}

test('standard color-blind symbols draw centered geometric paths without font fillText', () => {
  for (const symbol of STANDARD_OWNERSHIP_SYMBOLS) {
    assert.equal(isStandardOwnershipSymbol(symbol), true);
    const { canvas, calls } = mockCanvas();
    drawOwnershipSymbol(canvas, symbol, 32, 14);
    assert.equal(
      calls.some((call) => call.op === 'fillText'),
      false,
      `${symbol} should render via geometric path, not fillText`,
    );
    assert.equal(
      calls.some((call) => call.op === 'fill' || call.op === 'stroke' || call.op === 'fillRect'),
      true,
      `${symbol} must emit a draw command`,
    );
  }

  // Verify exact symmetry around (32, 32) for circle, square, diamond, plus, and cross.
  {
    const { canvas, calls } = mockCanvas();
    drawOwnershipSymbol(canvas, '●', 32, 14);
    const arc = calls.find((call) => call.op === 'arc');
    assert.ok(arc);
    assert.equal(arc.args[0], 32);
    assert.equal(arc.args[1], 32);
  }
  {
    const { canvas, calls } = mockCanvas();
    drawOwnershipSymbol(canvas, '■', 32, 14);
    const rect = calls.find((call) => call.op === 'fillRect');
    assert.ok(rect);
    const [x, y, w, h] = rect.args as number[];
    assert.equal(x + w / 2, 32);
    assert.equal(y + h / 2, 32);
  }
  {
    const { canvas, calls } = mockCanvas();
    drawOwnershipSymbol(canvas, '✚', 32, 14);
    const rects = calls.filter((call) => call.op === 'fillRect');
    assert.equal(rects.length, 2);
    for (const rect of rects) {
      const [x, y, w, h] = rect.args as number[];
      assert.equal(x + w / 2, 32);
      assert.equal(y + h / 2, 32);
    }
  }
  {
    const { canvas, calls } = mockCanvas();
    drawOwnershipSymbol(canvas, '◆', 32, 14);
    const points = calls
      .filter((call) => call.op === 'moveTo' || call.op === 'lineTo')
      .map((call) => call.args as [number, number]);
    assert.equal(points.length, 4);
    const avgX = points.reduce((sum, [x]) => sum + x, 0) / points.length;
    const avgY = points.reduce((sum, [, y]) => sum + y, 0) / points.length;
    assert.equal(avgX, 32);
    assert.equal(avgY, 32);
  }
});

test('non-standard symbols center using actualBoundingBox metrics and fall back safely when unavailable', () => {
  assert.equal(isStandardOwnershipSymbol('★'), false);
  {
    const { canvas, calls } = mockCanvas({
      actualBoundingBoxLeft: 14,
      actualBoundingBoxRight: 16,
      actualBoundingBoxAscent: 19,
      actualBoundingBoxDescent: 11,
    });
    drawOwnershipSymbol(canvas, '★', 32, 14);
    const fillText = calls.find((call) => call.op === 'fillText');
    assert.deepEqual(fillText?.args, ['★', 31, 36]);
  }
  {
    const { canvas, calls } = mockCanvas({});
    drawOwnershipSymbol(canvas, '⬢', 32, 14);
    const fillText = calls.find((call) => call.op === 'fillText');
    assert.deepEqual(fillText?.args, ['⬢', 32, 32]);
  }
});
