import "./style.css";

// @ts-expect-error: `__API_URL__` is filled in by vite-plugin-replace
const API_BASE_URL = __API_URL__;
const toText = (r: Response) => r.text();
const api = {
  submit: (n: number) =>
    fetch(API_BASE_URL + "/", { method: "POST", body: n.toString() })
      .then(toText)
      .then(parseInt),
  sync: () =>
    fetch(API_BASE_URL + "/")
      .then(toText)
      .then(parseInt),
};

const app = document.querySelector("#app")!;
app.innerHTML = `<div><span id="count">0</span><button id="click">Click me</button></div>`;

const el = {
  count: document.getElementById("count") as HTMLSpanElement,
  button: document.getElementById("click") as HTMLButtonElement,
};

let totalCount = 0;
let pendingCount = 0;

function setTotalCount(value: number) {
  totalCount = value;
  el.count.innerText = totalCount.toString();
}

function increment() {
  pendingCount += 1;
  setTotalCount(totalCount + 1);
}
async function synchronize() {
  let value = Math.min(pendingCount, 500);
  pendingCount -= value;

  setTotalCount(value > 0 ? await api.submit(value) : await api.sync());
}

el.button.onclick = increment;

// synchronize every 3 seconds + some random offset
setInterval(() => setTimeout(synchronize, Math.floor(Math.random() * 1000)), 3 * 1000);

api.sync().then((v) => setTotalCount(v));

