import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' };
const server = createServer(async (req, res) => {
  const name = req.url === '/' ? 'index.html' : req.url.slice(1);
  try {
    const body = await readFile(new URL(`site/${name}`, import.meta.url));
    res.writeHead(200, { 'Content-Type': types[name.slice(name.lastIndexOf('.'))] ?? 'application/octet-stream' }).end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise((ok) => server.listen(0, '127.0.0.1', ok));
const url = `http://127.0.0.1:${server.address().port}/`;

const cube = 'v -1 -1 -1\nv 1 -1 -1\nv 1 1 -1\nv -1 1 -1\nv -1 -1 1\nv 1 -1 1\nv 1 1 1\nv -1 1 1\n' +
  'f 1 4 3 2\nf 5 6 7 8\nf 1 2 6 5\nf 2 3 7 6\nf 3 4 8 7\nf 4 1 5 8\n';
const box = `; FBX 7.4.0 project file
FBXHeaderExtension:  {
	FBXVersion: 7400
}
GlobalSettings:  {
	Properties70:  {
		P: "UnitScaleFactor", "double", "Number", "",1
	}
}
Objects:  {
	Geometry: 1, "Geometry::box", "Mesh" {
		Vertices: *24 {
			a: -100,-100,-100,100,-100,-100,100,100,-100,-100,100,-100,-100,-100,100,100,-100,100,100,100,100,-100,100,100
		}
		PolygonVertexIndex: *24 {
			a: 0,3,2,-2,4,5,6,-8,0,1,5,-5,1,2,6,-6,2,3,7,-7,3,0,4,-8
		}
	}
	Model: 2, "Model::box", "Mesh" {
	}
	Material: 3, "Material::red", "" {
		Properties70:  {
			P: "DiffuseColor", "Color", "", "A",1,0,0
		}
	}
}
Connections:  {
	C: "OO",2,0
	C: "OO",1,2
	C: "OO",3,2
}
`;
const chunk = (id, body) => {
  const head = new DataView(new ArrayBuffer(6));
  head.setUint16(0, id, true);
  head.setUint32(2, 6 + body.length, true);
  return Buffer.concat([Buffer.from(head.buffer), body]);
};
const u16 = (...v) => Buffer.from(new Uint16Array(v).buffer);
const f32 = (...v) => Buffer.from(new Float32Array(v).buffer);
const square = chunk(0x4d4d, chunk(0x3d3d, chunk(0x4000, Buffer.concat([Buffer.from('floor\0'), chunk(0x4100, Buffer.concat([
  chunk(0x4110, Buffer.concat([u16(4), f32(-1, -1, 0, 1, -1, 0, 1, 1, 0, -1, 1, 0)])),
  chunk(0x4120, u16(2, 0, 1, 2, 0, 0, 2, 3, 0)),
]))]))));

const files = [
  { name: 'cube.obj', bytes: Buffer.from(cube), triangles: 12, red: false },
  { name: 'box.fbx', bytes: Buffer.from(box), triangles: 12, red: true },
  { name: 'square.3ds', bytes: square, triangles: 2, red: false },
];

const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const page = await browser.newPage({ viewport: { width: 640, height: 480 } });
const errors = [];
page.on('pageerror', (e) => (errors.push(e.message), console.log(`page error: ${e.message}`)));
page.on('console', (m) => console.log(`page: ${m.text()}`));
await page.goto(url);
await page.waitForFunction(() => window.viewer);
let failed = 0;
const background = await page.evaluate(() => window.viewer.middle());
for (const f of files) {
  await page.locator('#file').setInputFiles({ name: f.name, mimeType: 'application/octet-stream', buffer: f.bytes });
  await page.waitForFunction((n) => document.getElementById('info').textContent.startsWith(n), f.name);
  const [drawn, middle, text] = await page.evaluate(() => [window.viewer.drawn(), window.viewer.middle(), document.getElementById('info').textContent]);
  const shown = middle.some((c, i) => Math.abs(c - background[i]) > 8);
  const red = middle[0] > 2 * middle[1] && middle[0] > 2 * middle[2];
  const ok = drawn === f.triangles && shown && red === f.red;
  if (!ok) failed++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text} · ${drawn} drawn, the middle ${middle.slice(0, 3)}`);
}
await browser.close();
server.close();
process.exit(failed || errors.length ? 1 : 0);
