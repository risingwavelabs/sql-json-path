import * as wasm from './pkg/wasm.js';

const jsonInput = document.getElementById('json-input');
const jsonpathInput = document.getElementById('jsonpath-input');
const resultDisplay = document.getElementById('result');

wasm.default().then(() => {
    const evaluate = () => {
        try {
            const jsonPath = new wasm.JsonPath(jsonpathInput.value);
            const result = jsonPath.query(jsonInput.value);
            resultDisplay.textContent = result;
            resultDisplay.classList.remove('error-border');
            jsonpathInput.classList.remove('error-border');
            jsonPath.free();
        } catch (e) {
            resultDisplay.textContent = e;
            jsonpathInput.classList.add('error-border');
            resultDisplay.classList.add('error-border');
        }
    };

    evaluate();
    jsonInput.addEventListener('input', evaluate);
    jsonpathInput.addEventListener('input', evaluate);
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
jsonpathInput.value = examplePath;
