/**
 * @copyright https://github.com:Erkaman/gl-rock
 */

import Set from "es6-set";
import vec3 from "gl-vec3";

function getNeighbours(positions, cells) {
    /*
     adjacentVertices[i] contains a set containing all the indices of the neighbours of the vertex with
     index i.
     A set is used because it makes the algorithm more convenient.
     */
    let adjacentVertices = new Array(positions.length);

    // go through all faces.
    for (let iCell = 0; iCell < cells.length; ++iCell) {
        const cellPositions = cells[iCell];

        function wrap(i) {
            if (i < 0) {
                return cellPositions.length + i;
            } else {
                return i % cellPositions.length;
            }
        }

        // go through all the points of the face.
        for (let iPosition = 0; iPosition < cellPositions.length; ++iPosition) {
            // the neighbours of this points are the previous and next points(in the array)
            const cur = cellPositions[wrap(iPosition + 0)];
            const prev = cellPositions[wrap(iPosition - 1)];
            const next = cellPositions[wrap(iPosition + 1)];

            // create set on the fly if necessary.
            if (typeof adjacentVertices[cur] === "undefined") {
                adjacentVertices[cur] = new Set();
            }

            // add adjacent vertices.
            adjacentVertices[cur].add(prev);
            adjacentVertices[cur].add(next);
        }
    }

    // now we convert adjacentVertices from an array of sets, to an array of arrays.
    for (let i = 0; i < positions.length; ++i) {
        adjacentVertices[i] = Array.from(adjacentVertices[i]);
    }

    return { adjacentVertices: adjacentVertices };
}

/*
Projects the point `p` onto the plane defined by the normal `n` and the point `r0`
 */
function project(n, r0, p) {
    // For an explanation of the math, see http://math.stackexchange.com/a/100766

    let scratchVec = [0, 0, 0];
    let t = vec3.dot(n, vec3.subtract(scratchVec, r0, p)) / vec3.dot(n, n);

    const projectedP = [0, 0, 0];
    vec3.copy(projectedP, p);
    vec3.scaleAndAdd(projectedP, projectedP, n, t);

    return projectedP;
}

// scrape at vertex with index `positionIndex`.
const scrape = function (positionIndex, positions, cells, normals, adjacentVertices, strength, radius) {
    let traversed = new Array(positions.length);
    for (let i = 0; i < positions.length; ++i) {
        traversed[i] = false;
    }

    const centerPosition = positions[positionIndex];

    // to scrape, we simply project all vertices that are close to `centerPosition`
    // onto a plane. The equation of this plane is given by dot(n, r-r0) = 0,
    // where n is the plane normal, r0 is a point on the plane(in our case we set this to be the projected center),
    // and r is some arbitrary point on the plane.
    const n = normals[positionIndex];

    const r0 = [0, 0, 0];
    vec3.copy(r0, centerPosition);
    vec3.scaleAndAdd(r0, r0, n, -strength);

    const stack = [];
    stack.push(positionIndex);

    /*
     We use a simple flood-fill algorithm to make sure that we scrape all vertices around the center.
     This will be fast, since all vertices have knowledge about their neighbours.
     */
    while (stack.length > 0) {
        const topIndex = stack.pop();

        if (traversed[topIndex]) continue; // already traversed; look at next element in stack.
        traversed[topIndex] = true;

        const topPosition = positions[topIndex];
        // project onto plane.
        const p = [0, 0, 0];
        vec3.copy(p, topPosition);

        const projectedP = project(n, r0, p);

        if (vec3.squaredDistance(projectedP, r0) < radius) {
            positions[topIndex] = projectedP;
            normals[topIndex] = n;
        } else {
            continue;
        }

        const neighbourIndices = adjacentVertices[topIndex];
        for (let i = 0; i < neighbourIndices.length; ++i) {
            stack.push(neighbourIndices[i]);
        }
    }
};

export default {
    scrape,
    getNeighbours,
};
