// The diver: the Defendant as Style Lab's skinned mesh, animated by its baked
// clips (after Espiochum's skin.js). The manifest (data/mesh/diver.json) says
// where everything is in the mesh and animation files; the core says, each
// frame, which rows of the animation to blend for the lower and the upper
// body, the model matrix, the aim's bend and the grey and hurt mixes
// (game.actors(), laid out by numbers().diver). This file decides nothing:
// it uploads and draws.

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}

function link(gl, vertex, fragment) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertex));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragment));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  return program;
}

// files: { json(path), binary(path), image(path) }, each fetched from this origin
export async function makeSkin(gl, N, shaders, files) {
  const R = N.diver;
  const sceneVao = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
  const program = link(gl, shaders.vertex, shaders.fragment);
  const u = (name) => gl.getUniformLocation(program, name);
  const U = {
    vp: u("uVP"), model: u("uModel"), bend: u("uBend"), anim: u("uAnim"), texels: u("uTexels"),
    lower: u("uLower"), upper: u("uUpper"), seam: u("uSeam"), upperBones: u("uUpperBones"),
    tex: u("uTex"), hasTex: u("uHasTex"), tint: u("uTint"), surface: u("uSurface"),
    eye: u("uEye"), fx: u("uFx"), bubble: u("uBubble"), mix: u("uMix"), grey: u("uGrey"),
  };
  const sceneUniforms = N.uniforms.map(([name, size]) => ({ loc: u(name), size }));
  const loc = (name) => gl.getAttribLocation(program, name);
  const A = {
    position: { at: loc("aPos"), type: gl.FLOAT, norm: false, int: false },
    normal: { at: loc("aNormal"), type: gl.BYTE, norm: true, int: false },
    uv: { at: loc("aUV"), type: gl.FLOAT, norm: false, int: false },
    joints: { at: loc("aJoints"), type: gl.UNSIGNED_BYTE, norm: false, int: true },
    weights: { at: loc("aWeights"), type: gl.UNSIGNED_BYTE, norm: true, int: false },
  };
  const wide = Uint32Array.BYTES_PER_ELEMENT;

  const m = await files.json(R.model);
  const [bin, anim] = await Promise.all([files.binary(m.bin), files.binary(m.anim)]);

  async function texture(path) {
    const img = await files.image(path);
    const t = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.SRGB8_ALPHA8, gl.RGBA, gl.UNSIGNED_BYTE, img);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    return t;
  }

  const animTex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, animTex);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, m.anim_width, m.anim_rows, 0, gl.RGBA, gl.HALF_FLOAT, new Uint16Array(anim));
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);

  const parts = [];
  for (const p of m.parts) {
    if (p.vertices === 0) continue;
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const vb = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vb);
    gl.bufferData(gl.ARRAY_BUFFER, new Uint8Array(bin, p.offset, p.vertices * p.stride), gl.STATIC_DRAW);
    for (const [key, [size, offset]] of Object.entries(m.attrib)) {
      const a = A[key];
      if (a.at < 0) continue;
      gl.enableVertexAttribArray(a.at);
      if (a.int) gl.vertexAttribIPointer(a.at, size, a.type, p.stride, offset);
      else gl.vertexAttribPointer(a.at, size, a.type, a.norm, p.stride, offset);
    }
    const ib = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ib);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint8Array(bin, p.index_offset, p.indices * p.index_bytes), gl.STATIC_DRAW);
    gl.bindVertexArray(sceneVao);
    parts.push({
      vao,
      count: p.indices,
      type: p.index_bytes === wide ? gl.UNSIGNED_INT : gl.UNSIGNED_SHORT,
      tex: p.texture ? await texture(`mesh/tex/${p.texture}`) : null,
      tint: new Float32Array(p.tint),
      surface: new Float32Array([p.metal, p.rough, p.glass]),
    });
  }
  const upperBones = new Int32Array(m.upper);
  const blank = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, blank);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([1, 1, 1, 1]));

  let sceneValues = null;
  return {
    setUniforms(values) {
      sceneValues = values;
    },
    // after the scene's solids, before its ink and glow, with depth on
    draw(view, actors) {
      if (!actors.length) return;
      const prevProgram = gl.getParameter(gl.CURRENT_PROGRAM);
      const prevVao = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
      gl.useProgram(program);
      gl.disable(gl.BLEND);
      gl.enable(gl.DEPTH_TEST);
      gl.depthMask(true);
      gl.enable(gl.CULL_FACE);
      gl.cullFace(gl.BACK);
      const V = N.view;
      gl.uniformMatrix4fv(U.vp, false, view.subarray(...V.matrix));
      gl.uniform3fv(U.eye, view.subarray(...V.eye));
      gl.uniform4fv(U.fx, view.subarray(...V.fx));
      gl.uniform4fv(U.bubble, view.subarray(...V.bubble));
      if (sceneValues) {
        let at = 0;
        for (const s of sceneUniforms) {
          if (s.loc) gl[`uniform${s.size}fv`](s.loc, sceneValues.subarray(at, at + s.size));
          at += s.size;
        }
      }
      gl.uniform1i(U.anim, 0);
      gl.uniform1i(U.tex, 1);
      gl.uniform1i(U.texels, m.texels_per_bone);
      gl.uniform1i(U.seam, m.seam);
      gl.uniform1iv(U.upperBones, upperBones);
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, animTex);
      for (let i = 0; i < actors.length; i += R.values) {
        const rec = actors.subarray(i, i + R.values);
        if (!rec[R.shown]) continue;
        gl.uniform3fv(U.lower, rec.subarray(R.lower, R.lower + R.rows));
        gl.uniform3fv(U.upper, rec.subarray(R.upper, R.upper + R.rows));
        gl.uniformMatrix4x3fv(U.model, false, rec.subarray(...R.matrix));
        gl.uniformMatrix4x3fv(U.bend, false, rec.subarray(...R.bend));
        gl.uniform2f(U.mix, rec[R.grey], rec[R.hurt]);
        gl.uniform3fv(U.grey, rec.subarray(...R.grey_rgb));
        for (const p of parts) {
          gl.activeTexture(gl.TEXTURE0 + 1);
          gl.bindTexture(gl.TEXTURE_2D, p.tex || blank);
          gl.uniform1f(U.hasTex, p.tex ? 1 : 0);
          gl.uniform3fv(U.tint, p.tint);
          gl.uniform3fv(U.surface, p.surface);
          gl.bindVertexArray(p.vao);
          gl.drawElements(gl.TRIANGLES, p.count, p.type, 0);
        }
      }
      gl.activeTexture(gl.TEXTURE0);
      gl.bindVertexArray(prevVao);
      gl.useProgram(prevProgram);
    },
  };
}
