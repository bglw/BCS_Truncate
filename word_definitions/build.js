const path = require('path');
const fs = require('fs');
const readline = require('readline');
const jsonlines = require('jsonlines');

const input_file = path.join(__dirname, "kaikki.org-dictionary-English.json");

if (!fs.existsSync(input_file)) {
    console.error(`Need to build word definitions from a dinctionary reference.`);
    console.error(`Download the English JSON data from https://kaikki.org/dictionary/English/index.html`);
    console.error(`And place the file at ${input_file}`);
    process.exit(1);
}

const rl = readline.createInterface({
    input: fs.createReadStream(input_file),
    crlfDelay: Infinity
});
const parser = jsonlines.parse();

let written = 0;
let skipped = 0;
let writable = 0;

const words = {};

const writeWord = (word_json) => {
    // Skip words with casing, whitespace, or punctuation
    if (/[^a-z]/.test(word_json.word)) {
        skipped += 1;
        return;
    };

    const out_obj = {
        pos: word_json.pos,
        defs: word_json.senses.flatMap(sense => sense.raw_glosses || sense.glosses || [])
    };
    if (!out_obj.defs.length) {
        out_obj.defs.push(out_obj.etymology_text || "No definition found");
    }
    if (out_obj.defs.some(def => !def)) {
        console.error(`Bad def for ${word_json.word}:`);
        console.error(out_obj);
        console.error(JSON.stringify(word_json));
        process.exit(1);
    } else if (out_obj.defs.length === 0) {
        console.warn(`- - - - - - - - - - -`);
        console.warn(`No defs for ${word_json.word}`);
        console.warn(JSON.stringify(word_json, null, 2));
        console.warn(`- - - - - - - - - - -`);
    }

    const word_key = `${word_json.word}_tr`; // Fixes clash with `constructor`
    words[word_key] = words[word_key] || [];
    words[word_key].push(out_obj);

    written += 1;
    if (written % 5000 === 0) {
        console.log(`• Processed: ${written}, Skipped: ${skipped}`);
    }
}

parser.on('data', function (data) {
    writeWord(data);
});

rl.on('line', (line) => {
    writable += 1;
    parser.write(line);
    parser.write(`\n`);
});

rl.on('close', () => {
    parser.end();
});

parser.on('end', () => {
    console.log(`\n-------------\n`);

    console.log(`• Sorting words`);
    const keys = Object.keys(words).sort();

    console.log(`• Writing JSON lines`);
    var stream = fs.createWriteStream("defs.json");
    for (const key of keys) {
        stream.write(JSON.stringify([key.replace(/_tr$/, ''), words[key]]) + "\n");
    }
    stream.end();

    console.log(`\n-------------\n`);

    console.log(`• Ingested ${writable} words`);
    console.log(`• Processed ${written} words`);
    console.log(`• Skipped ${skipped} words`);
    console.log(`• Total processed ${written + skipped} words`);
    console.log(`• Total output ${keys.length} words`);
    if (written + skipped !== writable) {
        console.error(`ERR: Didn't process all words.`);
        process.exit(1);
    }
});
