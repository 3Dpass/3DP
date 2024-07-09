const RockObj = require("./miner-libs/rock_obj.js");
const Rock = require("./miner-libs/rock.js");
const randomArray = require("random-array");
const THREE = require("three");
const OBJExporter = import("three/examples/jsm/exporters/OBJExporter.js");
const yargs = require("yargs");
const { hideBin } = require("yargs/helpers");
const fs = require("fs");

const argv = yargs(hideBin(process.argv)).argv;
const host = argv.host || "127.0.0.1";
const port = argv.port || "9933";
const do_save = argv.save || false;
const apiUrl = `http://${host}:${port}`;
const filename = "rock.obj";

let interval = 3000;
const MIN_INTERVAL = 100;
const MAX_INTERVAL = 10000;
const ADJUSTMENT_PERCENT = 10;

let exporterModule;
let exporter;

void mining(do_save);

async function mining(do_save) {
  exporterModule = await OBJExporter;
  exporter = new exporterModule.OBJExporter();

  const rock = create_rock();
  const obj_file = create_obj_file(rock);
  if (do_save) {
    save(obj_file, filename);
    return;
  }
  fetch(apiUrl, {
    method: "POST",
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "poscan_pushMiningObject",
      params: [1, obj_file],
    }),
    headers: {
      "Content-Type": "application/json",
    },
  })
    .then((response) => response.json())
    .catch((e) => {
      console.log(e.toString());
    })
    .then((res) => {
      // example result: { jsonrpc: '2.0', result: 0, id: 1 }
      // Adjust interval based on result
      if (res && res.result === 0) {
        interval = Math.max(MIN_INTERVAL, Math.round(interval * (1 - ADJUSTMENT_PERCENT / 100)));
        console.log(`Decreased interval to ${interval}ms`);
      } else if (res && res.result === 1) {
        interval = Math.min(MAX_INTERVAL, Math.round(interval * (1 + ADJUSTMENT_PERCENT / 100)));
        console.log(`Increased interval to ${interval}ms`);
      }
      setTimeout(() => mining(), interval);
    });
}

function create_rock() {
  const rock_obj = new RockObj();
  rock_obj.seed = Math.round(randomArray(0, Number.MAX_SAFE_INTEGER).oned(1)[0]);
  rock_obj.varyMesh();
  return new Rock(rock_obj);
}

function create_obj_file(rock) {
  const scene = new THREE.Scene();

  const mesh = new THREE.Mesh(rock.geometry);
  scene.add(mesh);

  return exporter.parse(scene);
}

function save(text, filename) {
  fs.writeFile(filename, text, function (err) {
    if (err) {
      return console.log(err);
    }
    console.log("The file was saved!");
  });
}
