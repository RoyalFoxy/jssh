Object.defineProperty(globalThis, "${name}", {
  get() {
    return this.$getEnv("{name}");
  },
  set(newValue) {
    this.$setEnv("{name}", newValue);
  },
  configurable: true,
  enumerable: true,
});
