/**
 * @copyright https://github.com:Erkaman/gl-rock
 */

/*
Create UV sphere.
 */
const createSphere = function (opt) {
  let radius = opt.radius || 1.0;
  let stacks = opt.stacks || 32;
  let slices = opt.slices || 32;

  let vertices = [];
  let indexes = [];
  let normals = [];

  // keeps track of the index of the next vertex that we create.
  let index = 0;

  /*
   First of all, we create all the faces that are NOT adjacent to the
   bottom(0,-R,0) and top(0,+R,0) vertices of the sphere.

   (it's easier this way, because for the bottom and top vertices, we need to add triangle faces.
   But for the faces between, we need to add quad faces. )
   */

  // loop through the stacks.
  for (let i = 1; i < stacks; ++i) {
    let u = i / stacks;
    let phi = u * Math.PI;

    let stackBaseIndex = indexes.length / 2;

    // loop through the slices.
    for (let j = 0; j < slices; ++j) {
      let v = j / slices;
      let theta = v * (Math.PI * 2);

      let R = radius;
      // use spherical coordinates to calculate the positions.
      let x = Math.cos(theta) * Math.sin(phi);
      let y = Math.cos(phi);
      let z = Math.sin(theta) * Math.sin(phi);

      vertices.push([R * x, R * y, R * z]);
      normals.push([x, y, z]);

      if (i + 1 != stacks) {
        // for the last stack, we don't need to add faces.

        let i1, i2, i3, i4;

        if (j + 1 == slices) {
          // for the last vertex in the slice, we need to wrap around to create the face.
          i1 = index;
          i2 = stackBaseIndex;
          i3 = index + slices;
          i4 = stackBaseIndex + slices;
        } else {
          // use the indices from the current slice, and indices from the next slice, to create the face.
          i1 = index;
          i2 = index + 1;
          i3 = index + slices;
          i4 = index + slices + 1;
        }

        // add quad face
        indexes.push([i1, i2, i3]);
        indexes.push([i4, i3, i2]);
      }

      index++;
    }
  }

  /*
   Next, we finish the sphere by adding the faces that are adjacent to the top and bottom vertices.
   */

  let topIndex = index++;
  vertices.push([0.0, radius, 0.0]);
  normals.push([0, 1, 0]);

  let bottomIndex = index++;
  vertices.push([0, -radius, 0]);
  normals.push([0, -1, 0]);

  for (let i = 0; i < slices; ++i) {
    let i1 = topIndex;
    let i2 = i + 0;
    let i3 = (i + 1) % slices;
    indexes.push([i3, i2, i1]);

    i1 = bottomIndex;
    i2 = bottomIndex - 1 - slices + (i + 0);
    i3 = bottomIndex - 1 - slices + ((i + 1) % slices);
    indexes.push([i1, i2, i3]);
  }

  return { vertices: vertices, cells: indexes, normals: normals };
};

module.exports = createSphere;
