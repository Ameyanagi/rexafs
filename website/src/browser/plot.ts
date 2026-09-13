interface PlotData {
  x: Float64Array;
  y: Float64Array;
  title: string;
  xLabel: string;
  yLabel: string;
  zeroMinimum?: boolean;
}

const namespace = 'http://www.w3.org/2000/svg';

function element<K extends keyof SVGElementTagNameMap>(tag: K, attributes: Record<string, string | number>, text?: string): SVGElementTagNameMap[K] {
  const node = document.createElementNS(namespace, tag);
  for (const [key, value] of Object.entries(attributes)) node.setAttribute(key, String(value));
  if (text !== undefined) node.textContent = text;
  return node;
}

function label(value: number, precision = 5): string {
  if (value === 0 || Math.abs(value) < 1e-12) return '0';
  return Math.abs(value) >= 100000 || Math.abs(value) < 0.001
    ? value.toExponential(2)
    : Number(value.toPrecision(precision)).toString();
}

/**
 * Draw a spectrum with its physical axis labels. For large arrays, each group
 * of consecutive samples retains its endpoints and extrema, in sample order.
 * This limits SVG work while preserving narrow peaks; CSV export uses the full
 * arrays and never this display reduction.
 */
export function renderPlot(svg: SVGSVGElement, data: PlotData): void {
  const { x, y } = data;
  if (!x.length || x.length !== y.length) throw new Error('The selected result has no matching plot arrays.');
  let xmin = Infinity, xmax = -Infinity, ymin = Infinity, ymax = -Infinity;
  for (let index = 0; index < x.length; index++) {
    if (!Number.isFinite(x[index]) || !Number.isFinite(y[index])) throw new Error('The selected result contains a nonfinite plot value.');
    xmin = Math.min(xmin, x[index]); xmax = Math.max(xmax, x[index]);
    ymin = Math.min(ymin, y[index]); ymax = Math.max(ymax, y[index]);
  }
  const actualYmin = ymin, actualYmax = ymax;
  const yPadding = (ymax - ymin || Math.abs(ymax) || 1) * 0.07;
  ymin = data.zeroMinimum && ymin >= 0 ? 0 : ymin - yPadding;
  ymax += yPadding;
  if (xmax === xmin) { xmin -= 0.5; xmax += 0.5; }
  const width = Math.max(360, Math.round(svg.parentElement?.clientWidth || 900));
  const height = width < 550 ? 330 : 420;
  svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
  svg.replaceChildren(
    element('title', { id: 'plot-title' }, data.title),
    element('desc', { id: 'plot-description' }, `${x.length.toLocaleString()} samples. ${data.xLabel}: ${label(xmin)} to ${label(xmax)}. ${data.yLabel}: ${label(actualYmin)} to ${label(actualYmax)}.`),
  );
  const intervals = width < 550 ? 3 : 5;
  const yLabels = Array.from({ length: intervals + 1 }, (_, index) => label(ymin + (ymax - ymin) * index / intervals, width < 550 ? 3 : 5));
  const measure = element('text', { visibility: 'hidden' });
  svg.append(measure);
  let widestLabel = 0;
  for (const text of yLabels) {
    measure.textContent = text;
    widestLabel = Math.max(widestLabel, measure.getComputedTextLength());
  }
  measure.remove();
  const left = Math.max(width < 550 ? 82 : 80, Math.ceil(widestLabel) + 42);
  const right = 24, top = 22, bottom = 65;
  const plotWidth = width - left - right, plotHeight = height - top - bottom;
  const sx = (value: number) => left + (value - xmin) / (xmax - xmin) * plotWidth;
  const sy = (value: number) => top + (ymax - value) / (ymax - ymin) * plotHeight;
  for (let index = 0; index <= intervals; index++) {
    const xv = xmin + (xmax - xmin) * index / intervals;
    const yv = ymin + (ymax - ymin) * index / intervals;
    svg.append(
      element('line', { x1: sx(xv), y1: top, x2: sx(xv), y2: top + plotHeight, class: 'grid-line' }),
      element('line', { x1: left, y1: sy(yv), x2: left + plotWidth, y2: sy(yv), class: 'grid-line' }),
      element('text', { x: sx(xv), y: top + plotHeight + 23, 'text-anchor': 'middle' }, label(xv)),
      element('text', { x: left - 10, y: sy(yv) + 4, 'text-anchor': 'end' }, yLabels[index]),
    );
  }
  svg.append(
    element('line', { x1: left, y1: top + plotHeight, x2: left + plotWidth, y2: top + plotHeight, class: 'axis-line' }),
    element('line', { x1: left, y1: top, x2: left, y2: top + plotHeight, class: 'axis-line' }),
    element('text', { x: left + plotWidth / 2, y: height - 12, 'text-anchor': 'middle', class: 'axis-label' }, data.xLabel),
    element('text', { x: 18, y: top + plotHeight / 2, transform: `rotate(-90 18 ${top + plotHeight / 2})`, 'text-anchor': 'middle', class: 'axis-label' }, data.yLabel),
  );
  const groupSize = Math.max(1, Math.ceil(x.length / (plotWidth * 2)));
  const points: string[] = [];
  for (let start = 0; start < x.length; start += groupSize) {
    const end = Math.min(start + groupSize, x.length) - 1;
    let minimum = start, maximum = start;
    for (let index = start + 1; index <= end; index++) {
      if (y[index] < y[minimum]) minimum = index;
      if (y[index] > y[maximum]) maximum = index;
    }
    for (const index of [...new Set([start, minimum, maximum, end])].sort((a, b) => a - b)) {
      points.push(`${sx(x[index]).toFixed(2)},${sy(y[index]).toFixed(2)}`);
    }
  }
  svg.append(element('polyline', { points: points.join(' '), class: 'data-line' }));
}
