#version 300 es
// The diver: one skinned part (Style Lab's bake, tools/mesh/bake_diver.py).
// Bone matrices come from the animation texture: uTexels texels a bone, a
// 3x4 matrix in rows; one texture row a frame. The core sends two layers,
// each two rows and a blend (x, y, z): the lower layer for every bone; the
// bones flagged upper take the upper layer, set onto the lower body at the
// seam bone (their place relative to the seam from the upper clip, the
// seam's place from the lower clip) and bent for the aim (uBend, in the rest
// frame, about the pivot the core chose). The model matrix places the figure.
precision highp float;
precision highp int;
in vec3 aPos;
in vec3 aNormal;
in vec2 aUV;
in uvec4 aJoints;
in vec4 aWeights;
uniform mat4 uVP;
uniform mat4x3 uModel;
uniform mat4x3 uBend;
uniform highp sampler2D uAnim;
uniform int uTexels;
uniform vec3 uLower;
uniform vec3 uUpper;
uniform int uSeam;
uniform int uUpperBones[64];
out vec3 vWorld;
out vec3 vNormal;
out vec2 vUV;

mat4 bone(int row, int joint) {
  int x = joint * uTexels;
  vec4 r0 = texelFetch(uAnim, ivec2(x, row), 0);
  vec4 r1 = texelFetch(uAnim, ivec2(x + 1, row), 0);
  vec4 r2 = texelFetch(uAnim, ivec2(x + 2, row), 0);
  return mat4(r0.x, r1.x, r2.x, 0.0,
              r0.y, r1.y, r2.y, 0.0,
              r0.z, r1.z, r2.z, 0.0,
              r0.w, r1.w, r2.w, 1.0);
}

mat4 layer(vec3 rows, int joint) {
  return bone(int(rows.x), joint) * (1.0 - rows.z) + bone(int(rows.y), joint) * rows.z;
}

mat4 widen(mat4x3 m) {
  return mat4(vec4(m[0], 0.0), vec4(m[1], 0.0), vec4(m[2], 0.0), vec4(m[3], 1.0));
}

void main() {
  // the upper body's frame on the lower body: the seam where the lower
  // clip has it, bent, then back out of where the upper clip has it
  mat4 lift = layer(uLower, uSeam) * widen(uBend) * inverse(layer(uUpper, uSeam));
  mat4 s = mat4(0.0);
  for (int k = 0; k < 4; k++) {
    int j = int(aJoints[k]);
    mat4 m = uUpperBones[j] == 1 ? lift * layer(uUpper, j) : layer(uLower, j);
    s += m * aWeights[k];
  }
  vec4 local = s * vec4(aPos, 1.0);
  vec3 world = uModel * vec4(local.xyz, 1.0);
  vNormal = mat3(uModel) * (mat3(s) * aNormal);
  vUV = aUV;
  vWorld = world;
  gl_Position = uVP * vec4(world, 1.0);
}
