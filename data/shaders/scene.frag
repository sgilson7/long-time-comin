#version 300 es
precision highp float;
// Seven passes (the core says which): 0 solid, lit in flat bands like the
// TikZ art's fills; 1 facade, a panel whose windows are drawn here; 2 ink,
// the plum outline, flat; 3 glow, an additive halo that is brightest where
// you look through the middle of a shape; and three solids in other arts'
// manners: 4 one-bit dither (an engraving), 5 a PlayStation's few colours
// (its corners snapped in the vertex shader), 6 a photograph's negative.
in vec3 vWorld;
in vec3 vNormal;
in vec4 vColor;
in vec2 vFacade;
uniform vec3 uEye;
uniform vec3 uLightDir;
uniform vec3 uSky;
uniform vec3 uFog;
uniform vec2 uFogRange;
uniform vec3 uLight;
uniform vec3 uWindowDark;
uniform vec3 uInk;
uniform float uPass;
// See-through: the head (xyz) and the floor under it (w); the cut's sizes
// from balance.rs (near the lens, wide at the lens, wide by the head, how
// far short of the head it stops) and the share of its width cut solid;
// and whether this draw is scenery that may be cut.
uniform vec4 uFocus;
uniform vec4 uSee;
uniform float uSeeSolid;
uniform float uCut;
// x: how far the summit has melted (the Idiot God's coming), y: time
// stopped (the picture goes grey), z: how far space is bent (inside the
// god: colours drift), w: the clock in seconds
uniform vec4 uFx;
// a bubble of stopped time (centre, radius; radius 0: none): grey inside
uniform vec4 uBubble;
const float FACADE = 1.0;
const float INK = 2.0;
const float GLOW = 3.0;
const float DITHER = 4.0;
const float PS1 = 5.0;
const float NEGATIVE = 6.0;
const float PS1_LEVELS = 6.0;
// a facade's window grid: one window to a cell this wide and tall, in
// metres (crates/core/src/arena.rs: col_w and storey), and the window's
// half size and its frame's, as fractions of the cell
const vec2 CELL = vec2(2.4, 3.2);
const vec2 PANE = vec2(0.23, 0.235);
const vec2 FRAME = vec2(0.2, 0.21);
const float LIT = 0.45;
out vec4 outColor;
// the finished colour: grey while time is stopped, the eldritch tint as
// the summit melts
vec3 finish(vec3 c) {
  float grey = dot(c, vec3(0.3, 0.59, 0.11));
  float bubble = uBubble.w > 0.0 ? 1.0 - smoothstep(uBubble.w * 0.97, uBubble.w, length(vWorld - uBubble.xyz)) : 0.0;
  c = mix(c, vec3(grey) * vec3(0.85, 0.92, 1.1), max(uFx.y, bubble) * 0.85);
  if (uFx.z > 0.0) {
    float d = length(vWorld - uEye);
    c = mix(c, c.gbr, uFx.z * 0.3 * (0.5 + 0.5 * sin(uFx.w * 0.3 + d * 0.025)));
  }
  return mix(c, c * vec3(0.7, 0.55, 1.15) + vec3(0.05, 0.12, 0.02) * uFx.x, uFx.x * 0.6);
}
// the 4×4 ordered-dither threshold at this pixel
float bayer() {
  vec2 p = mod(floor(gl_FragCoord.xy), 4.0);
  float b = mod(p.x + p.y * 2.0, 4.0) * 4.0 + mod(p.x * 2.0 + p.y, 4.0);
  return (b + 0.5) / 16.0;
}
// Is this fragment of scenery between the camera and the player? Inside the
// cut's core it goes; at its rim every other pixel goes, which reads as glass.
bool seeThrough() {
  if (uCut < 0.5 || uPass == GLOW) return false;
  vec3 axis = uFocus.xyz - uEye;
  float len = length(axis);
  if (len < uSee.x) return false;
  // the floor at and below the feet always stays
  if (vWorld.y < uFocus.w + 0.15) return false;
  vec3 rel = vWorld - uEye;
  if (length(rel) < uSee.x) return true;
  vec3 dir = axis / len;
  float t = dot(rel, dir);
  if (t <= 0.0 || t > len - uSee.w) return false;
  float wide = mix(uSee.y, uSee.z, t / len);
  float off = length(rel - dir * t);
  if (off > wide) return false;
  if (off < wide * uSeeSolid) return true;
  vec2 cell = floor(gl_FragCoord.xy);
  return mod(cell.x + cell.y, 2.0) < 1.0;
}
void main() {
  if (seeThrough()) discard;
  vec3 n = normalize(vNormal);
  float d = length(vWorld - uEye);
  float f = clamp((d - uFogRange.x) / (uFogRange.y - uFogRange.x), 0.0, 1.0);
  f = f * f * (3.0 - 2.0 * f);
  if (uPass == FACADE) {
    vec2 g = vFacade / CELL;
    vec2 at = abs(fract(g) - 0.5);
    if (at.x > PANE.x || at.y > PANE.y) discard;
    vec2 cell = floor(g);
    float h = fract(sin(dot(cell + vColor.a * 97.0, vec2(12.9898, 78.233))) * 43758.5453);
    bool lit = h < LIT;
    vec3 c = lit ? vColor.rgb * 1.2 + vec3(0.06) : uWindowDark;
    if (at.x > FRAME.x || at.y > FRAME.y) c = uInk;
    outColor = vec4(finish(mix(c, uFog, f * (lit ? 0.5 : 1.0))), 1.0);
    return;
  }
  if (uPass == GLOW) {
    vec3 v = normalize(uEye - vWorld);
    float through = abs(dot(n, v));
    float a = pow(through, 2.2) * vColor.a * (1.0 - 0.8 * f);
    outColor = vec4(finish(vColor.rgb * a), 1.0);
    return;
  }
  if (uPass == INK) {
    outColor = vec4(finish(mix(vColor.rgb, uFog, f * 0.85)), 1.0);
    return;
  }
  vec3 base = vColor.rgb;
  float diffuse = dot(n, uLightDir);
  float band = diffuse > 0.55 ? 1.0 : (diffuse > 0.12 ? 0.72 : 0.48);
  float sky = 0.5 + 0.5 * n.y;
  vec3 ambient = mix(uFog * 0.55, uSky * 0.8, sky);
  vec3 lit = base * (ambient * 0.75 + uLight * band * 0.5);
  vec3 glow = base * 1.25 + vec3(0.08);
  lit = mix(lit, glow, vColor.a);
  if (uPass == DITHER) {
    float l = dot(lit, vec3(0.3, 0.59, 0.11)) / max(dot(base, vec3(0.3, 0.59, 0.11)), 0.05);
    lit = l * 0.75 > bayer() ? base : base * 0.12;
  } else if (uPass == PS1) {
    lit = floor(lit * PS1_LEVELS + 0.5) / PS1_LEVELS;
  } else if (uPass == NEGATIVE) {
    lit = vec3(1.0) - lit;
  }
  outColor = vec4(finish(mix(lit, uFog, f * (1.0 - 0.6 * vColor.a))), 1.0);
}
