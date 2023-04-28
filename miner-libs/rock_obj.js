/**
 * @copyright https://github.com:Erkaman/gl-rock
 */
const randomArray = require("random-array");

/*
RockObj contains all the parameters that are used when generating a rock.
 */
class RockObj {
  constructor() {
    this.seed = 100;
    this.meshNoiseScale = { val: 1.0 };
    this.meshNoiseStrength = { val: 0.2 };
    this.scrapeCount = { val: 7 };
    this.scrapeMinDist = { val: 0.8 };
    this.scrapeStrength = { val: 0.2 };
    this.scrapeRadius = { val: 0.3 };
    this.scale = [1.0, 1.0, 1.0];
    this.varyStrength = 1.0;
  }

  varyParameter(param, variance, min, max) {
    param.val += randomArray(-variance * this.varyStrength, +variance * this.varyStrength).oned(
      1
    )[0];
    if (param.val > max) param.val = max;
    if (param.val < min) param.val = min;
  }

  varyArray(arr, i, variance, min, max) {
    arr[i] += randomArray(-variance * this.varyStrength, +variance * this.varyStrength).oned(1)[0];
    if (arr[i] > max) arr[i] = max;
    if (arr[i] < min) arr[i] = min;
  }

  randomizeMesh() {
    this.meshNoiseScale.val = randomArray(MESH_NOISE_SCALE_MIN, MESH_NOISE_SCALE_MAX).oned(1)[0];
    this.meshNoiseStrength.val = randomArray(MESH_NOISE_STRENGTH_MIN, MESH_NOISE_STRENGTH_MAX).oned(
      1
    )[0];

    this.scrapeCount.val = Math.floor(randomArray(SCRAPE_COUNT_MIN, SCRAPE_COUNT_MAX).oned(1)[0]);
    this.scrapeMinDist.val = randomArray(SCRAPE_MIN_DIST_MIN, SCRAPE_MIN_DIST_MAX).oned(1)[0];

    this.scrapeStrength.val = randomArray(SCRAPE_STRENGTH_MIN, SCRAPE_STRENGTH_MAX).oned(1)[0];

    this.scrapeRadius.val = randomArray(SCRAPE_RADIUS_MIN, SCRAPE_RADIUS_MAX).oned(1)[0];

    this.scale = randomArray(SCALE_MIN, SCALE_MAX).oned(3);
  }

  varyMesh() {
    this.varyParameter(
      this.meshNoiseScale,
      MESH_NOISE_SCALE_VARY,
      MESH_NOISE_SCALE_MIN,
      MESH_NOISE_SCALE_MAX
    );
    this.varyParameter(
      this.meshNoiseStrength,
      MESH_NOISE_STRENGTH_VARY,
      MESH_NOISE_STRENGTH_MIN,
      MESH_NOISE_STRENGTH_MAX
    );

    this.varyParameter(this.scrapeCount, SCRAPE_COUNT_VARY, SCRAPE_COUNT_MIN, SCRAPE_COUNT_MAX);

    this.varyParameter(
      this.scrapeMinDist,
      SCRAPE_MIN_DIST_VARY,
      SCRAPE_MIN_DIST_MIN,
      SCRAPE_MIN_DIST_MAX
    );
    this.varyParameter(
      this.scrapeStrength,
      SCRAPE_STRENGTH_VARY,
      SCRAPE_STRENGTH_MIN,
      SCRAPE_STRENGTH_MAX
    );

    this.varyParameter(this.scrapeRadius, SCRAPE_RADIUS_VARY, SCRAPE_RADIUS_MIN, SCRAPE_RADIUS_MAX);

    const scale = this.scale;
    this.varyArray(scale, 0, SCALE_VARY, SCALE_MIN, SCALE_MAX);
    this.varyArray(scale, 1, SCALE_VARY, SCALE_MIN, SCALE_MAX);
    this.varyArray(scale, 2, SCALE_VARY, SCALE_MIN, SCALE_MAX);
  }
}

const MESH_NOISE_SCALE_MIN = 0.5;
const MESH_NOISE_SCALE_MAX = 5.0;
const MESH_NOISE_SCALE_VARY = 0.1;

const MESH_NOISE_STRENGTH_MIN = 0.0;
const MESH_NOISE_STRENGTH_MAX = 0.5;
const MESH_NOISE_STRENGTH_VARY = 0.05;

const SCRAPE_COUNT_MIN = 0;
const SCRAPE_COUNT_MAX = 15;
const SCRAPE_COUNT_VARY = 2;

const SCRAPE_MIN_DIST_MIN = 0.1;
const SCRAPE_MIN_DIST_MAX = 1.0;
const SCRAPE_MIN_DIST_VARY = 0.05;

const SCRAPE_STRENGTH_MIN = 0.1;
const SCRAPE_STRENGTH_MAX = 0.6;
const SCRAPE_STRENGTH_VARY = 0.05;

const SCRAPE_RADIUS_MIN = 0.1;
const SCRAPE_RADIUS_MAX = 0.5;
const SCRAPE_RADIUS_VARY = 0.05;

const SCALE_MIN = +1.0;
const SCALE_MAX = +2.0;
const SCALE_VARY = +0.1;

module.exports = RockObj;
