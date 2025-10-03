#! (shebang test)
import * as std from "std";
import * as os from "os";

function assert(actual, expected, message) {
    if (arguments.length == 1)
        expected = true;

    if (Object.is(actual, expected))
        return;

    if (actual !== null && expected !== null
        && typeof actual == 'object' && typeof expected == 'object'
        && actual.toString() === expected.toString())
        return;

    throw Error("assertion failed: got |" + actual + "|" +
        ", expected |" + expected + "|" +
        (message ? " (" + message + ")" : ""));
}

// load more elaborate version of assert if available
try { std.loadScript("assert.js"); } catch (e) { }

/*----------------*/

function test_printf() {
    assert(std.sprintf("a=%d s=%s", 123, "abc"), "a=123 s=abc");
    assert(std.sprintf("%010d", 123), "0000000123");
    assert(std.sprintf("%x", -2), "fffffffe");
    assert(std.sprintf("%lx", -2), "fffffffffffffffe");
    assert(std.sprintf("%10.1f", 2.1), "       2.1");
    assert(std.sprintf("%*.*f", 10, 2, -2.13), "     -2.13");
    assert(std.sprintf("%#lx", 0x7fffffffffffffffn), "0x7fffffffffffffff");
}

function test_file1() {
    var f, len, str, size, buf, ret, i, str1;

    f = std.tmpfile();
    str = "hello world\n";
    f.puts(str);

    f.seek(0, std.SEEK_SET);
    str1 = f.readAsString();
    assert(str1 === str);

    f.seek(0, std.SEEK_END);
    size = f.tell();
    assert(size === str.length);

    f.seek(0, std.SEEK_SET);

    buf = new Uint8Array(size);
    ret = f.read(buf.buffer, 0, size);
    assert(ret === size);
    for (i = 0; i < size; i++)
        assert(buf[i] === str.charCodeAt(i));

    f.close();
}

function test_file2() {
    var f, str, i, size;
    f = std.tmpfile();
    str = "hello world\n";
    size = str.length;
    for (i = 0; i < size; i++)
        f.putByte(str.charCodeAt(i));
    f.seek(0, std.SEEK_SET);
    for (i = 0; i < size; i++) {
        assert(str.charCodeAt(i) === f.getByte());
    }
    assert(f.getByte() === -1);
    f.close();
}

function test_getline() {
    var f, line, line_count, lines, i;

    lines = ["hello world", "line 1", "line 2"];
    f = std.tmpfile();
    for (i = 0; i < lines.length; i++) {
        f.puts(lines[i], "\n");
    }

    f.seek(0, std.SEEK_SET);
    assert(!f.eof());
    line_count = 0;
    for (; ;) {
        line = f.getline();
        if (line === null)
            break;
        assert(line == lines[line_count]);
        line_count++;
    }
    assert(f.eof());
    assert(line_count === lines.length);

    f.close();
}

function test_ext_json() {
    var expected, input, obj;
    expected = '{"x":false,"y":true,"z2":null,"a":[1,8,160],"b":"abc\\u000bd","s":"str"}';
    input = `{ "x":false, /*comments are allowed */
               "y":true,  // also a comment
               z2:null, // unquoted property names
               "a":[+1,0o10,0xa0,], // plus prefix, octal, hexadecimal
               "b": "ab\
c\\vd", // multi-line strings, '\v' escape
               "s":'str',} // trailing comma in objects and arrays, single quoted string
            `;
    obj = std.parseExtJSON(input);
    assert(JSON.stringify(obj), expected);

    obj = std.parseExtJSON('[Infinity, +Infinity, -Infinity, NaN, +NaN, -NaN, .1, -.2]');
    assert(obj[0], Infinity);
    assert(obj[1], Infinity);
    assert(obj[2], -Infinity);
    assert(obj[3], NaN);
    assert(obj[4], NaN);
    assert(obj[5], NaN);
    assert(obj[6], 0.1);
    assert(obj[7], -0.2);
}

function test_timer() {
    var th, i;

    /* just test that a timer can be inserted and removed */
    th = [];
    for (i = 0; i < 3; i++)
        th[i] = os.setTimeout(function () { }, 1000);
    for (i = 0; i < 3; i++)
        os.clearTimeout(th[i]);
}

/* test closure variable handling when freeing asynchronous
   function */
function test_async_gc() {
    (async function run() {
        let obj = {}

        let done = () => {
            obj
            std.gc();
        }

        Promise.resolve().then(done)

        const p = new Promise(() => { })

        await p
    })();
}

/* check that the promise async rejection handler is not invoked when
   the rejection is handled not too late after the promise
   rejection. */
function test_async_promise_rejection() {
    var counter = 0;
    var p1, p2, p3;
    p1 = Promise.reject();
    p2 = Promise.reject();
    p3 = Promise.resolve();
    p1.catch(() => counter++);
    p2.catch(() => counter++);
    p3.then(() => counter++)
    os.setTimeout(() => { assert(counter, 3) }, 10);
}

test_printf();
test_file1();
test_file2();
test_getline();
test_timer();
test_ext_json();
test_async_gc();
test_async_promise_rejection();

console.log("ok");
