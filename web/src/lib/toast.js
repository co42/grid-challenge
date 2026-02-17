let nextId = 0;
let listener = null;

export const toasts = {
  _items: [],

  subscribe(fn) {
    listener = fn;
    fn(this._items);
    return () => {
      listener = null;
    };
  },

  show(message, type = 'success', duration = 3000) {
    const id = nextId++;
    this._items = [...this._items, { id, message, type }];
    listener?.(this._items);
    setTimeout(() => this.dismiss(id), duration);
  },

  dismiss(id) {
    this._items = this._items.filter((t) => t.id !== id);
    listener?.(this._items);
  },
};

export function toast(message, type = 'success') {
  toasts.show(message, type);
}
