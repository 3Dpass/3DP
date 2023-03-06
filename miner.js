import RockObj from "./miner-libs/rock_obj.js";
import Rock from "./miner-libs/rock.js";
import randomArray from "random-array";
import * as THREE from "three";
import { OBJExporter } from "three/examples/jsm/exporters/OBJExporter.js";
import axios from "axios";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import fs from "fs";

const argv = yargs(hideBin(process.argv)).argv;
const interval = argv.interval || 1000;
const host = argv.host || "localhost";
const port = argv.port || "9933";
const do_save = argv.save || false;
const apiUrl = `http://${host}:${port}`;
const filename = "rock.obj";

mining(do_save);

function mining(do_save) {
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
  rock_obj.scale = [1.0, 1.0, 2.0];
  return new Rock(rock_obj);
}

function create_obj_file(rock) {
  const scene = new THREE.Scene();

  const mesh = new THREE.Mesh(rock.geometry);
  scene.add(mesh);

  const exporter = new OBJExporter();
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
