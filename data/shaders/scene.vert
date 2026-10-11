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
// scenery melts as the Idiot God comes (x of uFx); bodies do not
uniform float uCut;
uniform vec2 uFx;
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
}
