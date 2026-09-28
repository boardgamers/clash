export class MapGesture {
  private pointers = new Set<number>();
  private origin = [0, 0];
  private moved = false;

  get dragging() {
    return this.pointers.size > 0 && this.moved;
  }

  down(id: number, x: number, y: number) {
    if (!this.pointers.size) {
      this.origin = [x, y];
      this.moved = false;
    }
    this.pointers.add(id);
    if (this.pointers.size > 1) this.moved = true;
  }

  move(x: number, y: number) {
    if (this.pointers.size && Math.hypot(x - this.origin[0], y - this.origin[1]) > 5) this.moved = true;
  }

  up(id: number, x: number, y: number) {
    this.move(x, y);
    const wasDown = this.pointers.delete(id);
    return wasDown && !this.moved && this.pointers.size === 0;
  }

  cancel(id: number) {
    this.pointers.delete(id);
    this.moved = true;
  }
}
