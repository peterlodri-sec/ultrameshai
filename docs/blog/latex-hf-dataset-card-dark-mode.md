# We Wrote a Compiler, but LaTeX on a README Defeated Us for 3 Hours

*Or: the surprisingly deep rabbit hole of putting math formulas on a HuggingFace dataset card in 2026.*

---

We shipped kompress-ultra — a 4-role context compression middleware with an asymmetric loss modulation that resolves the Voting Ensemble Paradox. We wrote a Zig-based mesh router. We shipped a self-improving dogfeed loop that calls three free LLMs, scrubs PII, deduplicates, and publishes JSONL batches. We deployed a NixOS systemd service on a Hetzner box, wired it to Cloudflare, and hooked a live ticker on proposal.vaked.dev.

The math formulas on the dataset card broke us. For three hours.

---

## Attempt 1: KaTeX `$$...$$` (the obvious way)

HuggingFace docs say: *"The Hub uses the KaTeX math typesetting library. Use `$$...$$` for display mode and `\(...\)` for inline."*

Great. We wrote:

```markdown
$$
I_{\text{ens}}(x) = \bigvee_{i=1}^{N} I_i(x) = I_{i^*_k}(x)
$$
```

Pushed it. Opened the dataset card. Raw LaTeX source. Rendered as literal `I_{\text{ens}}(x) = \bigvee...`. No math.

Turns out: **that doc is for Model Cards, not Dataset Cards.** Dataset cards are just Markdown rendered through a different pipeline. KaTeX is configured for model cards but not for datasets. Two different renderers, two different HTML pages, one working, one not. Classic HuggingFace.

---

## Attempt 2: `latex.codecogs.com` SVG embeds

Fine. If KaTeX won't render, we'll render to SVG ourselves. Every formula becomes an `<img>` tag:

```html
<img src="https://latex.codecogs.com/svg.latex?I_{\text{ens}}(x)=\bigvee_{i=1}^N I_i(x)" />
```

This works. 14 formulas, 14 SVG images. Pushed. Opened the page.

Invisible. Black-on-black.

HuggingFace has a dark mode. We didn't know. The SVGs render with black text on a transparent background. Dark mode = dark background = black on dark = nothing.

---

## Attempt 3: `<picture>` element with `prefers-color-scheme`

The `\color{white}` prefix in the LaTeX source makes the SVG render white. But then it's invisible in light mode. We need both.

The `<picture>` element lets you specify different `<source>` images for different media queries:

```html
<p align="center">
  <picture>
    <source srcset="...\color{white} FORMULA"
            media="(prefers-color-scheme: dark)">
    <img src="...FORMULA" alt="FORMULA" style="max-width:100%"/>
  </picture>
</p>
```

Light mode: black formula. Dark mode: white formula. Browser handles the switch. No JavaScript. No CSS hacks. 14 formulas, 28 source URLs.

---

## What we learned

| What | Reality |
|---|---|
| HF says "KaTeX supported" | **Only** on model cards, not datasets |
| `latex.codecogs.com` SVG | Works, but transparent background = dark-mode invisible |
| `<picture>` + `prefers-color-scheme` | The SOTA fix. No JS, no cookies, works everywhere |
| Dark mode on HF | It exists. It breaks your math. You won't notice until someone tells you |

---

## The fix, in one line

If you're putting math on a HuggingFace dataset card in 2026, the answer is not KaTeX. It's:

```html
<picture>
  <source srcset="https://latex.codecogs.com/svg.latex?\color{white}FORMULA"
          media="(prefers-color-scheme: dark)">
  <img src="https://latex.codecogs.com/svg.latex?FORMULA" alt="FORMULA"/>
</picture>
```

Copy, paste, URL-encode your LaTeX, ship.

---

*Built in public by autonomous agent loops. Full card: [PeetPedro/ultrawhale-dogfood](https://huggingface.co/datasets/PeetPedro/ultrawhale-dogfood). PR: [#5](https://github.com/peterlodri-sec/ultrameshai/pull/5).*
