const {html, svg} = htl;

// update every 10 seconds
const INTERVAL = 10 * 1000;
// graphs show 5 minutes worth of data
const SCALE = 5 * 60 * 1000;

const WIDTH = 350;
const HEIGHT = 200;
const MARGIN = {
  top: 5,
  right: 35,
  bottom: 20,
  left: 5
};

function drawGraph(name, data) {
  const now = Date.now();

  const x = d3.scaleUtc().range([MARGIN.left, WIDTH - MARGIN.right]).domain([
    now - SCALE, now
  ]);

  const y = d3.scaleLinear()
                .range([HEIGHT - MARGIN.bottom, MARGIN.top])
                .domain([0, d3.max(data, (d) => d[1])])
                .nice();

  const xAxis = (g) =>
      g.attr('transform', `translate(0,${HEIGHT - MARGIN.bottom})`)
          .call(d3.axisBottom(x).ticks(d3.timeMinute.every(1)));
  const yAxis = (g) =>
      g.attr('transform', `translate(${WIDTH - MARGIN.right},0)`)
          .call(d3.axisRight(y).ticks(5));

  const line = d3.line().x((d) => x(d[0])).y((d) => y(d[1]));

  return html`<svg class="graph" viewBox="0 0 ${WIDTH} ${HEIGHT}">
    <text
      class="value"
      x="${WIDTH - MARGIN.right - 5}"
      y="${HEIGHT - MARGIN.bottom - 50}"
    >
      ${y.tickFormat()(data[data.length - 1][1])}
    </text>
    <text
      class="title"
      x="${WIDTH - MARGIN.right - 5}"
      y="${HEIGHT - MARGIN.bottom - 10}"
    >
      ${name}
    </text>
    <path class="line" d="${line(data)}"></path>
    ${d3.select(svg`<g>`).call(xAxis).node()}
    ${d3.select(svg`<g>`).call(yAxis).node()}
  </svg>`;
}

class Graph {
  constructor(name, data) {
    this.name = name;
    this.data = [data];
    this.elem = drawGraph(this.name, this.data);
  }

  push(data) {
    while (this.data[0] && this.data[0][0] < Date.now() - SCALE) {
      this.data.shift();
    }
    this.data.push(data);

    // TODO: create graph once, and then refresh only updated data
    const elem = drawGraph(this.name, this.data);
    this.elem.parentNode.replaceChild(elem, this.elem);
    this.elem = elem;
  }
}

class Metrics {
  constructor() {
    this.elem = html`<div class="graphs"></div>`;
    this.metrics = undefined;
  }

  async update() {
    const {metrics} = await (await fetch(`data.json?${Date.now()}`)).json();
    const timestamp = Date.now();

    if (!this.metrics) {
      this.metrics = {};
      for (const name of Object.keys(metrics)) {
        const graph = (this.metrics[name] = new Graph(name, [
                         timestamp,
                         metrics[name],
                       ]));
        this.elem.appendChild(graph.elem);
      }
    } else {
      for (const name of Object.keys(this.metrics)) {
        this.metrics[name].push([timestamp, metrics[name]]);
      }
    }
  }
}

async function main() {
  const metrics = new Metrics();
  document.querySelector('.main').appendChild(metrics.elem);
  metrics.update();
  setInterval(() => metrics.update(), INTERVAL);
}

main();
