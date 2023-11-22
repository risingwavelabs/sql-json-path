import * as wasm from './pkg/wasm.js';

const jsonInput = document.getElementById('json-input');
const jsonpathQuery = document.getElementById('jsonpath-query');
const resultDisplay = document.getElementById('result');

wasm.default().then(() => {
    const evaluate = () => {
        try {
            const jsonPath = new wasm.JsonPath(jsonpathQuery.value);
            const result = jsonPath.query(jsonInput.value);
            resultDisplay.textContent = result;
            jsonPath.free();
        } catch (e) {
            resultDisplay.textContent = 'Error: ' + e.message;
        }
    };

    evaluate();
    jsonInput.addEventListener('input', evaluate);
    jsonpathQuery.addEventListener('input', evaluate);
});

const examplePath = "($.store.book[*] ?(@.price < 10)).title";
const exampleJson = {
    "store": {
        "book": [
            {
                "category": "reference",
                "author": "Nigel Rees",
                "title": "Sayings of the Century",
                "price": 8.95
            },
            {
                "category": "fiction",
                "author": "Evelyn Waugh",
                "title": "Sword of Honour",
                "price": 12.99
            },
            {
                "category": "fiction",
                "author": "Herman Melville",
                "title": "Moby Dick",
                "isbn": "0-553-21311-3",
                "price": 8.99
            },
            {
                "category": "fiction",
                "author": "J. R. R. Tolkien",
                "title": "The Lord of the Rings",
                "isbn": "0-395-19395-8",
                "price": 22.99
            }
        ],
        "bicycle": {
            "color": "red",
            "price": 19.95
        }
    },
    "expensive": 10
};
jsonInput.value = JSON.stringify(exampleJson, null, 2);
jsonpathQuery.value = examplePath;
