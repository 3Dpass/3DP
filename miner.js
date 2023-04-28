const RockObj = require("./miner-libs/rock_obj.js");
const Rock = require("./miner-libs/rock.js");
const randomArray = require("random-array");
const THREE = require("three");
const OBJExporter = import("three/examples/jsm/exporters/OBJExporter.js");
const axios = require("axios");
const yargs = require("yargs");
const { hideBin } = require("yargs/helpers");
const fs = require("fs");

const argv = yargs(hideBin(process.argv)).argv;
const interval = argv.interval || 1000;
const host = argv.host || "127.0.0.1";
const port = argv.port || "9933";
const do_save = argv.save || false;
const apiUrl = `http://${host}:${port}`;
const filename = "rock.obj";

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
  axios
    .post(apiUrl, {
      jsonrpc: "2.0",
      id: 1,
      method: "poscan_pushMiningObject",
      params: [1, obj_file],
    })
    .catch((e) => {
      console.log(e.toString());
    })
    .then((response) => {
      if (response && response.hasOwnProperty("data")) {
        console.log(response.data);
      }
      setTimeout(mining, interval);
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
