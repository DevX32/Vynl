<script lang="ts">
  import {
    eqBands,
    eqMaxDb,
    eqMinDb,
    eqResponseDb,
  } from "@lib/eq.svelte";

  interface Props {
    bands: number[];
    disabled?: boolean;
  }

  const { bands, disabled = false }: Props = $props();

  const W = 320;
  const H = 96;
  const PAD_X = 2;
  const PAD_Y = 7;

  const F_MIN = 20;
  const F_MAX = 20000;
  const LOG_MIN = Math.log10(F_MIN);
  const LOG_SPAN = Math.log10(F_MAX) - LOG_MIN;

  const GRID_FREQS = [100, 1000, 10000];
  const STEP = 6;

  const isFlat = $derived(bands.every((g) => Math.abs(g) < 0.05));

  function x(freq: number): number {
    const t = (Math.log10(freq) - LOG_MIN) / LOG_SPAN;
    return PAD_X + t * (W - PAD_X * 2);
  }

  const points = $derived.by(() => {
    const out: { f: number; db: number }[] = [];
    for (let i = 0; i <= 240; i++) {
      const f = Math.pow(10, LOG_MIN + (LOG_SPAN * i) / 240);
      out.push({ f, db: eqResponseDb(bands, f) });
    }
    return out;
  });

  const domain = $derived.by(() => {
    const floor = Math.max(Math.abs(eqMinDb()), Math.abs(eqMaxDb()));
    let peak = 0;
    for (const p of points) peak = Math.max(peak, Math.abs(p.db));
    const span = Math.max(floor, Math.ceil(peak / STEP) * STEP);
    return { hi: span, lo: -span };
  });

  function yIn(db: number, d: { hi: number; lo: number }): number {
    const t = (d.hi - db) / (d.hi - d.lo);
    return PAD_Y + t * (H - PAD_Y * 2);
  }

  const path = $derived.by(() => {
    let d = "";
    points.forEach((p, i) => {
      d += `${i === 0 ? "M" : "L"}${x(p.f).toFixed(2)},${yIn(p.db, domain).toFixed(2)}`;
    });
    return d;
  });

  const areaPath = $derived.by(() => {
    const base = yIn(0, domain);
    return `${path}L${x(F_MAX).toFixed(2)},${base}L${x(F_MIN).toFixed(2)},${base}Z`;
  });

  const ticks = $derived(
    eqBands()
      .map((freq, i) => ({
        freq,
        gain: bands[i] ?? 0,
        cx: x(freq),
        cy: yIn(eqResponseDb(bands, freq), domain),
      }))
      .filter((m) => Math.abs(m.gain) > 0),
  );

  const gridDbs = $derived.by(() => {
    const out: number[] = [];
    for (let db = domain.lo; db <= domain.hi; db += STEP) out.push(db);
    return out;
  });

  function pct(db: number): string {
    return `${((yIn(db, domain) / H) * 100).toFixed(2)}%`;
  }
</script>

<div class="wrap" class:dimmed={disabled}>
  <svg
    class="curve"
    viewBox={`0 0 ${W} ${H}`}
    preserveAspectRatio="none"
    role="img"
    aria-label="Response curve"
  >
    <defs>
      <linearGradient id="eq-curve-fill" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0%" stop-color="var(--accent)" stop-opacity="0.26" />
        <stop offset="100%" stop-color="var(--accent)" stop-opacity="0.02" />
      </linearGradient>
    </defs>

    {#each GRID_FREQS as freq (freq)}
      <line
        class="grid"
        x1={x(freq)}
        x2={x(freq)}
        y1={PAD_Y}
        y2={H - PAD_Y}
      />
    {/each}

    {#each gridDbs as db (db)}
      <line
        class="rule"
        class:zero={db === 0}
        x1={PAD_X}
        x2={W - PAD_X}
        y1={yIn(db, domain)}
        y2={yIn(db, domain)}
      />
    {/each}

    {#if !isFlat}
      <path class="fill" d={areaPath} />
      <path class="line" d={path} />
      {#each ticks as m (m.freq)}
        <line
          class="tick"
          x1={m.cx}
          x2={m.cx}
          y1={m.cy - 3.5}
          y2={m.cy + 3.5}
        />
      {/each}
    {/if}
  </svg>

  <span class="axis" style:top={pct(domain.hi)}>+{domain.hi}</span>
  <span class="axis" style:top={pct(0)}>0</span>
  <span class="axis" style:top={pct(domain.lo)}>{domain.lo}</span>
</div>

<style>
  .wrap {
    position: relative;
    height: 96px;
    transition: opacity 0.2s;
  }

  .wrap.dimmed {
    opacity: 0.35;
  }

  .curve {
    width: 100%;
    height: 96px;
    display: block;
  }

  .grid {
    stroke: var(--line-strong);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .rule {
    stroke: var(--line);
    stroke-width: 1;
    vector-effect: non-scaling-stroke;
  }

  .rule.zero {
    stroke: var(--line-strong);
    stroke-dasharray: 2 3;
  }

  .fill {
    fill: url(#eq-curve-fill);
    stroke: none;
  }

  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
  }

  .tick {
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linecap: round;
    vector-effect: non-scaling-stroke;
    opacity: 0.85;
  }

  .axis {
    position: absolute;
    right: 3px;
    transform: translateY(-50%);
    font-family: var(--font-mono);
    font-size: 8.5px;
    line-height: 1;
    color: var(--faint);
    opacity: 0.55;
    pointer-events: none;
  }
</style>