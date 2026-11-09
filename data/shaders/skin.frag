#version 300 es
// The diver's light: lifelike, not the world's bands and ink. The zone's own
// sun, sky and fog (the scene's uniforms), smooth diffuse, a specular lobe
// by roughness and a metal's response (copper, brass, lead and iron take
// their colour into the highlight), and the sky reflected in metal and the
// helmet's glass, which is drawn dark, glossy and opaque. Then the core's
// mixes: grey as the Eraser takes you, white for a moment when hit; the
// scene's fog; and the scene's finish (grey while time is stopped, the
// eldritch tints).
precision highp float;
in vec3 vWorld;
in vec3 vNormal;
in vec2 vUV;
uniform sampler2D uTex;
uniform float uHasTex;
uniform vec3 uTint;
// metalness, roughness, glass
uniform vec3 uSurface;
uniform vec3 uLightDir;
uniform vec3 uSky;
uniform vec3 uFog;
uniform vec2 uFogRange;
uniform vec3 uLight;
uniform vec3 uEye;
uniform vec4 uFx;
uniform vec4 uBubble;
// how grey, how hurt; the grey it goes to
uniform vec2 uMix;
uniform vec3 uGrey;
out vec4 outColor;

const float PI = 3.14159265;
const vec3 GLASS = vec3(0.012, 0.016, 0.02);

vec3 toLinear(vec3 c) { return pow(max(c, vec3(0.0)), vec3(2.2)); }

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

void main() {
  vec3 tint = toLinear(uTint);
  vec3 base = mix(tint, texture(uTex, vUV).rgb * tint, uHasTex);
  float metal = uSurface.x;
  float rough = clamp(uSurface.y, 0.06, 1.0);
  float glass = uSurface.z;
  if (glass > 0.5) {
    base = GLASS;
    metal = 0.0;
    rough = 0.06;
  }
  vec3 n = normalize(vNormal);
  if (!gl_FrontFacing) n = -n;
  vec3 l = normalize(uLightDir);
  vec3 v = normalize(uEye - vWorld);
  vec3 h = normalize(l + v);
  vec3 sun = toLinear(uLight);
  vec3 sky = toLinear(uSky);
  vec3 fog = toLinear(uFog);
  float ndl = max(dot(n, l), 0.0);
  float ndv = max(dot(n, v), 0.001);
  float ndh = max(dot(n, h), 0.0);
  // diffuse: the sun, and the scene's own sky-and-fog ambient
  vec3 ambient = mix(fog * 0.55, sky * 0.8, 0.5 + 0.5 * n.y);
  vec3 diffuse = base * (1.0 - metal) * (ambient * 0.8 + sun * ndl * 0.85);
  // specular (GGX, Schlick), and the sky in what reflects
  vec3 f0 = mix(vec3(0.04), base, metal);
  float a = rough * rough;
  float dd = ndh * ndh * (a * a - 1.0) + 1.0;
  float ggx = a * a / (PI * dd * dd);
  vec3 fh = f0 + (1.0 - f0) * pow(1.0 - max(dot(h, v), 0.0), 5.0);
  vec3 spec = ggx * fh * 0.25 * sun * ndl;
  vec3 r = reflect(-v, n);
  vec3 env = mix(fog * 0.5, sky * 1.1, smoothstep(-0.3, 0.7, r.y));
  vec3 fv = f0 + (1.0 - f0) * pow(1.0 - ndv, 5.0) * (1.0 - rough);
  vec3 lit = diffuse + spec + env * fv * (1.0 - rough * 0.6);
  lit = mix(lit, toLinear(uGrey), uMix.x);
  lit = mix(lit, vec3(1.0), uMix.y);
  vec3 c = pow(lit, vec3(1.0 / 2.2));
  float d = length(vWorld - uEye);
  float f = clamp((d - uFogRange.x) / (uFogRange.y - uFogRange.x), 0.0, 1.0);
  f = f * f * (3.0 - 2.0 * f);
  outColor = vec4(finish(mix(c, uFog, f)), 1.0);
}
