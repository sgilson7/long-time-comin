#version 300 es
// One shape: the mesh vertex, placed by the instance's three columns and
// translation (iA, iB, iC; translation in .w), coloured by iD (rgb, glow).
in vec3 aPos;
in vec3 aNormal;
in vec4 iA;
in vec4 iB;
in vec4 iC;
in vec4 iD;
uniform mat4 uVP;
// scenery melts as the Idiot God comes (x of uFx); bodies do not. Inside
// the god (z of uFx: how far) space curls up round the eye in the distance
// and breathes with the clock (w, seconds)
uniform float uCut;
uniform vec4 uFx;
uniform vec3 uEye;
// the pass: 5 snaps corners to a coarse grid on screen, as a PlayStation did
uniform float uPass;
const float INK = 2.0;
const float GLOW = 3.0;
const float PS1 = 5.0;
// A tie in depth with a solid (a shape's outline hull lying in another's
// face, a glowing strip laid in a wall) flickers as the camera moves. The
// ink is drawn a few depth steps back, so the solid wins the tie; a halo a
// few forward, so it shows on the surface it lies in. In clip space, as a
// share of w: about eight steps of a 24-bit depth buffer.
const float INK_BIAS = 1.0e-6;
const float HALO_BIAS = 1.0e-6;
const vec2 PS1_GRID = vec2(160.0, 120.0);
// near the eye space is flat, so aiming is as it looks; each shape is
// lifted as one piece
const float FLAT_NEAR = 18.0;
const float CURL = 0.0035;
const float BREATHE = 0.025;
out vec3 vWorld;
out vec3 vNormal;
out vec4 vColor;
// a facade's position on its panel, in metres: along the face, up it
out vec2 vFacade;
void main() {
  vec3 w = iA.xyz * aPos.x + iB.xyz * aPos.y + iC.xyz * aPos.z + vec3(iA.w, iB.w, iC.w);
  float melt = uFx.x * uFx.x * uCut;
  w.y -= melt * (18.0 + 9.0 * sin(w.x * 0.37 + w.z * 0.21));
  w.x += melt * 3.0 * sin(w.y * 0.5 + w.z * 0.3);
  if (uFx.z > 0.0) {
    // by the shape's centre, so a shape bends as one piece: a floor is
    // not tilted by its far corners
    vec3 centre = vec3(iA.w, iB.w, iC.w);
    float far = max(length(centre.xz - uEye.xz) - FLAT_NEAR, 0.0);
    float t = uFx.w;
    w.y += uFx.z * far * far * CURL;
    w += uFx.z * far * BREATHE * vec3(sin(t * 0.7 + w.z * 0.11), sin(t * 0.5 + w.x * 0.09), cos(t * 0.6 + w.y * 0.1));
  }
  // the columns carry a non-uniform scale, so normals take the inverse
  // scale: divide each axis by its column's squared length
  vec3 n = iA.xyz * (aNormal.x / max(dot(iA.xyz, iA.xyz), 1e-8))
         + iB.xyz * (aNormal.y / max(dot(iB.xyz, iB.xyz), 1e-8))
         + iC.xyz * (aNormal.z / max(dot(iC.xyz, iC.xyz), 1e-8));
  vNormal = n;
  vWorld = w;
  vColor = iD;
  vFacade = vec2((aPos.z + 0.5) * length(iC.xyz), (aPos.x + 0.5) * length(iA.xyz));
  gl_Position = uVP * vec4(w, 1.0);
  if (uPass == INK) gl_Position.z += INK_BIAS * gl_Position.w;
  if (uPass == GLOW) gl_Position.z -= HALO_BIAS * gl_Position.w;
  if (uPass == PS1) {
    gl_Position.xy = floor(gl_Position.xy / gl_Position.w * PS1_GRID + 0.5) / PS1_GRID * gl_Position.w;
  }
}
