const DEFAULT_CONCURRENCY = 6;

export class CoverLoader {
  constructor(api, concurrency = DEFAULT_CONCURRENCY) {
    this.api = api;
    this.concurrency = concurrency;
    this.active = 0;
    this.queue = [];
    this.cache = new Map();
    this.missing = new Set();
    this.pending = new Map();
  }

  load(image, itemPath) {
    if (this.missing.has(itemPath)) return;

    const cached = this.cache.get(itemPath);
    if (cached) {
      this.#show(image, cached);
      return;
    }

    let request = this.pending.get(itemPath);
    if (!request) {
      request = new Promise((resolve, reject) => {
        this.queue.push({ itemPath, resolve, reject });
        this.#drain();
      });
      this.pending.set(itemPath, request);
    }
    request.then((url) => this.#show(image, url)).catch(() => {});
  }

  dispose() {
    for (const url of this.cache.values()) URL.revokeObjectURL(url);
    this.cache.clear();
    this.missing.clear();
  }

  #drain() {
    while (this.active < this.concurrency && this.queue.length) {
      const task = this.queue.shift();
      this.active += 1;
      this.#fetch(task.itemPath)
        .then(task.resolve, (error) => {
          if (error.status === 404) this.missing.add(task.itemPath);
          task.reject(error);
        })
        .finally(() => {
          this.active -= 1;
          this.pending.delete(task.itemPath);
          this.#drain();
        });
    }
  }

  async #fetch(itemPath) {
    const response = await this.api(
      `/api/thumbnail?item=${encodeURIComponent(itemPath)}`,
    );
    const url = URL.createObjectURL(await response.blob());
    this.cache.set(itemPath, url);
    return url;
  }

  #show(image, url) {
    if (!image.isConnected) return;
    image.src = url;
    image.onload = () => image.classList.add("ready");
  }
}
