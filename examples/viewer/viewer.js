const canvas = document.getElementById('view');
const info = document.getElementById('info');
const gl = canvas.getContext('webgl2', { antialias: true });
if (!gl) {
  info.textContent = 'This browser has no WebGL 2.';
  throw new Error('no WebGL 2');
}
const { instance } = await WebAssembly.instantiateStreaming(fetch('load3d_viewer.wasm'));
const wasm = instance.exports;

const program = (() => {
  const shader = (type, text) => {
    const s = gl.createShader(type);
    gl.shaderSource(s, text);
    gl.compileShader(s);
    return s;
  };
  const p = gl.createProgram();
  gl.attachShader(p, shader(gl.VERTEX_SHADER, `#version 300 es
    in vec3 position; in vec3 color; uniform mat4 viewProj; out vec3 world; out vec3 tint;
    void main() { world = position; tint = color; gl_Position = viewProj * vec4(position, 1.0); }`));
  // each triangle's normal from the slopes of its position across the screen, so no normals need to be sent
  gl.attachShader(p, shader(gl.FRAGMENT_SHADER, `#version 300 es
    precision highp float; in vec3 world; in vec3 tint; uniform vec3 eye; out vec4 outColor;
    void main() {
      vec3 n = normalize(cross(dFdx(world), dFdy(world)));
      float light = 0.3 + 0.7 * abs(dot(n, normalize(eye - world)));
      outColor = vec4(pow(tint * light, vec3(1.0 / 2.2)), 1.0);
    }`));
  gl.linkProgram(p);
  return p;
})();

const model = { vao: gl.createVertexArray(), count: 0, center: [0, 0, 0], radius: 1 };
const camera = { yaw: 0.6, pitch: 0.35, distance: 3 };

function show(name, bytes) {
  // the pointers first: a call that grows the wasm's memory leaves a view taken before it empty
  const at = wasm.alloc(bytes.length);
  new Uint8Array(wasm.memory.buffer, at, bytes.length).set(bytes);
  const triangles = wasm.load();
  const [text, textLen] = [wasm.message(), wasm.message_len()];
  const message = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, text, textLen));
  if (triangles === 0) {
    info.textContent = `${name}: ${message || 'no triangles'}`;
    model.count = 0;
    draw();
    return;
  }
  const [p, c] = [wasm.positions(), wasm.colors()];
  const positions = new Float32Array(wasm.memory.buffer, p, 9 * triangles).slice();
  const perTriangle = new Float32Array(wasm.memory.buffer, c, 3 * triangles);
  const colors = new Float32Array(9 * triangles);
  for (let t = 0; t < triangles; t++) for (let k = 0; k < 3; k++) colors.set(perTriangle.subarray(3 * t, 3 * t + 3), 9 * t + 3 * k);
  const lo = [Infinity, Infinity, Infinity], hi = [-Infinity, -Infinity, -Infinity];
  for (let i = 0; i < positions.length; i++) {
    lo[i % 3] = Math.min(lo[i % 3], positions[i]);
    hi[i % 3] = Math.max(hi[i % 3], positions[i]);
  }
  model.center = lo.map((l, i) => (l + hi[i]) / 2);
  model.radius = Math.max(1e-6, Math.hypot(...hi.map((h, i) => h - lo[i])) / 2);
  camera.distance = 2.5 * model.radius;
  gl.bindVertexArray(model.vao);
  for (const [name, data] of [['position', positions], ['color', colors]]) {
    gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
    const at = gl.getAttribLocation(program, name);
    gl.enableVertexAttribArray(at);
    gl.vertexAttribPointer(at, 3, gl.FLOAT, false, 0, 0);
  }
  model.count = 3 * triangles;
  const size = hi.map((h, i) => Number((h - lo[i]).toPrecision(3)).toLocaleString('en-US')).join(' × ');
  const unit = wasm.metres() ? 'm' : "in the file's units";
  info.textContent = `${name} · ${triangles.toLocaleString('en-US')} triangles · ${size} ${unit} · ${message}`;
  draw();
}

function viewProj() {
  const { yaw, pitch, distance } = camera;
  const eye = [Math.cos(pitch) * Math.sin(yaw), Math.sin(pitch), Math.cos(pitch) * Math.cos(yaw)].map((d, i) => model.center[i] + distance * d);
  const sub = (a, b) => a.map((x, i) => x - b[i]);
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const unit = (a) => a.map((x) => x / Math.hypot(...a));
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const f = unit(sub(model.center, eye)), s = unit(cross(f, [0, 1, 0])), u = cross(s, f);
  const near = Math.max(distance - 2 * model.radius, distance / 1000), far = distance + 2 * model.radius;
  const t = 1 / Math.tan(Math.PI / 8), aspect = canvas.width / canvas.height;
  const proj = [t / aspect, 0, 0, 0, 0, t, 0, 0, 0, 0, (far + near) / (near - far), -1, 0, 0, (2 * far * near) / (near - far), 0];
  const view = [s[0], u[0], -f[0], 0, s[1], u[1], -f[1], 0, s[2], u[2], -f[2], 0, -dot(s, eye), -dot(u, eye), dot(f, eye), 1];
  const m = new Float32Array(16);
  for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) for (let k = 0; k < 4; k++) m[4 * c + r] += proj[4 * k + r] * view[4 * c + k];
  return { m, eye };
}

function draw() {
  const scale = window.devicePixelRatio || 1;
  canvas.width = Math.round(canvas.clientWidth * scale);
  canvas.height = Math.round(canvas.clientHeight * scale);
  gl.viewport(0, 0, canvas.width, canvas.height);
  gl.clearColor(0.125, 0.14, 0.165, 1);
  gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
  if (model.count === 0) return;
  gl.enable(gl.DEPTH_TEST);
  gl.useProgram(program);
  const { m, eye } = viewProj();
  gl.uniformMatrix4fv(gl.getUniformLocation(program, 'viewProj'), false, m);
  gl.uniform3fv(gl.getUniformLocation(program, 'eye'), eye);
  gl.bindVertexArray(model.vao);
  gl.drawArrays(gl.TRIANGLES, 0, model.count);
}

async function open(file) {
  show(file.name, new Uint8Array(await file.arrayBuffer()));
}

document.getElementById('file').addEventListener('change', (e) => e.target.files[0] && open(e.target.files[0]));
canvas.addEventListener('dragover', (e) => (e.preventDefault(), document.body.classList.add('dropping')));
canvas.addEventListener('dragleave', () => document.body.classList.remove('dropping'));
canvas.addEventListener('drop', (e) => {
  e.preventDefault();
  document.body.classList.remove('dropping');
  if (e.dataTransfer.files[0]) open(e.dataTransfer.files[0]);
});
canvas.addEventListener('pointermove', (e) => {
  if (!e.buttons) return;
  camera.yaw -= e.movementX * 0.01;
  camera.pitch = Math.max(-1.5, Math.min(1.5, camera.pitch + e.movementY * 0.01));
  draw();
});
canvas.addEventListener('wheel', (e) => {
  e.preventDefault();
  camera.distance = Math.max(model.radius * 0.05, camera.distance * Math.exp(e.deltaY * 0.001));
  draw();
}, { passive: false });
window.addEventListener('resize', draw);
// for the browser test (test.mjs): the triangles last drawn, and the colour drawn at the middle of the view
window.viewer = {
  show,
  drawn: () => model.count / 3,
  middle: () => {
    draw();
    const rgba = new Uint8Array(4);
    gl.readPixels(canvas.width >> 1, canvas.height >> 1, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, rgba);
    return [...rgba];
  },
};
draw();
