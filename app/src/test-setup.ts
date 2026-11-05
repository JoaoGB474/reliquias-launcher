import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";
afterEach(cleanup);

// O jsdom não implementa a Web Animations API que as transições do Svelte usam.
if (typeof Element !== "undefined" && !Element.prototype.animate) {
  Element.prototype.animate = function () {
    const alvo = {
      onfinish: null as null | (() => void),
      cancel() {},
      finish() {},
      play() {},
      pause() {},
      reverse() {},
      finished: Promise.resolve(),
    };
    queueMicrotask(() => alvo.onfinish?.());
    return alvo as unknown as Animation;
  };
}
