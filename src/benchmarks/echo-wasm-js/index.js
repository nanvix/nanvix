// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.


//==================================================================================================
// Constants
//==================================================================================================

const MAX_REQUEST_SIZE = 4096;
const STDIN = 0;
const STDOUT = 1;

//==================================================================================================
// Main
//==================================================================================================

const input = readInput();
writeOutput(input);

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Read input from stdin.
function readInput() {
    const chunkSize = MAX_REQUEST_SIZE;
    const inputChunks = [];
    let totalBytes = 0;

    // Read all the available bytes.
    while (1) {
        const buffer = new Uint8Array(chunkSize);
        const bytesRead = Javy.IO.readSync(STDIN, buffer);

        totalBytes += bytesRead;
        if (bytesRead === 0) {
            break;
        }
        inputChunks.push(buffer.subarray(0, bytesRead));
    }

    // Assemble input into a single Uint8Array
    const { finalBuffer } = inputChunks.reduce((context, chunk) => {
        context.finalBuffer.set(chunk, context.bufferOffset);
        context.bufferOffset += chunk.length;
        return context;
    }, { bufferOffset: 0, finalBuffer: new Uint8Array(totalBytes) });

    return new TextDecoder().decode(finalBuffer);
}

// Write output to stdout.
function writeOutput(output) {
    const encodedOutput = new TextEncoder().encode(output);
    const buffer = new Uint8Array(encodedOutput);
    Javy.IO.writeSync(STDOUT, buffer);
}
