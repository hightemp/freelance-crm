export interface EditMenuRequest {
  x: number;
  y: number;
  target: HTMLElement;
}

let request = $state.raw<EditMenuRequest | null>(null);

export function currentEditMenu(): EditMenuRequest | null {
  return request;
}

export function openEditMenu(at: EditMenuRequest): void {
  request = at;
}

export function closeEditMenu(): void {
  request = null;
}
