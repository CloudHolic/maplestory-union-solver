import js from "@eslint/js";
import { defineConfig, globalIgnores } from "eslint/config";
import { createTypeScriptImportResolver } from "eslint-import-resolver-typescript";
import betterTailwindcss from "eslint-plugin-better-tailwindcss";
import importX from "eslint-plugin-import-x";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import stylistic from "@stylistic/eslint-plugin";
import globals from "globals";
import tseslint from "typescript-eslint";

export default defineConfig([
	globalIgnores(["dist", "wasm-pkg"]),
	{
		files: ["**/*.{ts,tsx}"],
		extends: [
			js.configs.recommended,
			tseslint.configs.recommended,
			reactHooks.configs.flat.recommended,
			reactRefresh.configs.vite,
			importX.flatConfigs.recommended,
			importX.flatConfigs.typescript
		],
		plugins: {
			"@stylistic": stylistic,
			"better-tailwindcss": betterTailwindcss
		},
		languageOptions: {
			globals: globals.browser
		},
		settings: {
			"import-x/resolver-next": [
				createTypeScriptImportResolver({
					alwaysTryTypes: true,
					project: "./tsconfig.app.json"
				})
			],
			"better-tailwindcss": {
				entryPoint: "src/index.css"
			}
		},
		rules: {
			"import-x/order": [
				"error",
				{
					groups: [
						"builtin",
						"external",
						"internal",
						["parent", "sibling", "index"]
					],
					pathGroups: [
						{
							pattern: "@solver/wasm",
							group: "external",
							position: "after"
						},
						{
							pattern: "@/**",
							group: "internal"
						}
					],
					pathGroupsExcludedImportTypes: ["builtin"],
					"newlines-between": "always",
					alphabetize: {
						order: "asc",
						caseInsensitive: true
					}
				}
			],
			"import-x/newline-after-import": "error",
			"import-x/no-duplicates": "error",
			"import-x/no-unresolved": [
				"error",
				{
					ignore: ["\\?worker$", "\\?url$", "\\?raw$"]
				}
			],
			"import-x/default": "off",
			"sort-imports": [
				"error",
				{
					ignoreCase: true,
					ignoreDeclarationSort: true,
					ignoreMemberSort: false
				}
			],
			curly: ["error", "multi"],
			"@stylistic/nonblock-statement-body-position": ["error", "below"],
			"@stylistic/brace-style": ["error", "1tbs", { allowSingleLine: false }],
			"@stylistic/indent": ["error", "tab", { SwitchCase: 1 }],
			"@stylistic/no-tabs": ["error", { allowIndentationTabs: true }],
			"@stylistic/no-trailing-spaces": "error",
			"@stylistic/no-multiple-empty-lines": [
				"error",
				{
					max: 1,
					maxEOF: 0,
					maxBOF: 0
				}
			],
			"@stylistic/eol-last": ["error", "always"],
			"@stylistic/quotes": [
				"error",
				"double",
				{
					avoidEscape: true,
					allowTemplateLiterals: "always"
				}
			],
			"@stylistic/jsx-quotes": ["error", "prefer-double"],
			"@stylistic/semi": ["error", "always"],
			"@stylistic/comma-dangle": ["error", "never"],
			"@stylistic/object-curly-spacing": ["error", "always"],
			"@stylistic/array-bracket-spacing": ["error", "never"],
			"@stylistic/space-infix-ops": "error",
			"@stylistic/keyword-spacing": [
				"error",
				{
					before: true,
					after: true
				}
			],
			"@stylistic/space-before-blocks": "error",
			"@stylistic/space-before-function-paren": [
				"error",
				{
					anonymous: "always",
					named: "never",
					asyncArrow: "always"
				}
			],
			"@stylistic/space-in-parens": ["error", "never"],
			"@stylistic/comma-spacing": [
				"error",
				{
					before: false,
					after: true
				}
			],
			"@stylistic/arrow-parens": ["error", "as-needed"],
			"@stylistic/arrow-spacing": [
				"error",
				{
					before: true,
					after: true
				}
			],
			"better-tailwindcss/enforce-consistent-class-order": "error",
			"better-tailwindcss/no-duplicate-classes": "warn",
			"better-tailwindcss/no-conflicting-classes": "error",
			"better-tailwindcss/no-unnecessary-whitespace": "warn",
			"better-tailwindcss/no-unknown-classes": "off"
		}
	}
]);
