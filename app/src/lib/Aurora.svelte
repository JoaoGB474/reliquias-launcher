<script lang="ts">
  import { onMount } from "svelte";
  // Aurora de pixels: um campo gerado (não uma ilustração) numa grade pequena, com dithering de Bayer,
  // ampliada sem suavizar. Ouro nasce na rachadura (à esquerda), ametista domina o resto.
  let { aberto = false }: { aberto?: boolean } = $props();
  const L = 176;
  const A = 128;
  const BAYER = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
  const AMETISTA = [
    [10, 7, 19],
    [24, 15, 52],
    [45, 28, 102],
    [78, 50, 160],
    [120, 82, 205],
    [168, 132, 238],
  ];
  const OURO = [
    [10, 7, 19],
    [48, 31, 14],
    [98, 66, 22],
    [160, 112, 34],
    [214, 164, 62],
    [244, 214, 128],
  ];
  let canvas: HTMLCanvasElement;
  let forca = 0.9; // sobe suavemente quando a cena "abre"
  $effect(() => {
    forca = aberto ? 1.25 : 0.9;
  });

  onMount(() => {
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    const img = ctx.createImageData(L, A);
    const reduz = matchMedia("(prefers-reduced-motion: reduce)").matches;
    let t0 = performance.now();
    let ultimo = 0;
    let raf = 0;
    let suave = forca;
    const nivel = (v: number, i: number) => {
      const lim = BAYER[i] / 16;
      return Math.max(0, Math.min(5, Math.floor(v * 5 + lim)));
    };
    function quadro(agora: number) {
      const t = (agora - t0) / 1000;
      suave += (forca - suave) * 0.04;
      const d = img.data;
      for (let y = 0; y < A; y++) {
        const ny = y / A;
        for (let x = 0; x < L; x++) {
          const nx = x / L;
          // plasma lento de baixa frequência, vindo do canto superior direito
          const p =
            Math.sin(nx * 5.2 + t * 0.22) * 0.5 +
            Math.sin(ny * 6.4 - t * 0.17 + nx * 2.1) * 0.5 +
            Math.sin((nx + ny) * 4.1 + t * 0.13) * 0.35;
          const foco = Math.max(0, 1 - Math.hypot(nx - 0.85, ny - 0.0) * 1.15);
          let am = (0.2 + 0.18 * p + foco * 0.9) * suave;
          // ouro sangra a partir da rachadura, mais forte embaixo
          const borda = Math.max(0, 1 - nx * 3.2);
          let og = borda * (0.25 + 0.75 * ny) * (0.7 + 0.3 * Math.sin(t * 0.5 + ny * 3)) * suave;
          am = Math.max(0, Math.min(1, am));
          og = Math.max(0, Math.min(1, og));
          const i = (y % 4) * 4 + (x % 4);
          const la = nivel(am, i);
          const lo = nivel(og, i);
          const c = lo > la ? OURO[lo] : AMETISTA[la];
          const k = (y * L + x) * 4;
          d[k] = c[0];
          d[k + 1] = c[1];
          d[k + 2] = c[2];
          d[k + 3] = 255;
        }
      }
      ctx!.putImageData(img, 0, 0);
    }
    function laco(agora: number) {
      raf = requestAnimationFrame(laco);
      if (document.hidden || agora - ultimo < 66) return; // ~15 quadros/s: leve e com cara de pixel
      ultimo = agora;
      quadro(agora);
    }
    quadro(t0);
    if (!reduz) raf = requestAnimationFrame(laco);
    return () => cancelAnimationFrame(raf);
  });
</script>

<canvas bind:this={canvas} width={L} height={A} aria-hidden="true"></canvas>

<style>
  canvas {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    image-rendering: pixelated;
    pointer-events: none;
  }
</style>
