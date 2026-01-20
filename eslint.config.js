import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';

export default ts.config(
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs['flat/recommended'],
	{
		languageOptions: {
			globals: {
				...globals.browser,
				...globals.node
			}
		}
	},
	{
		files: ['**/*.svelte', '**/*.svelte.ts'],
		languageOptions: {
			parserOptions: {
				projectService: true,
				extraFileExtensions: ['.svelte'],
				parser: ts.parser
			}
		}
	},
	{
		ignores: [
			'build/',
			'.svelte-kit/',
			'node_modules/',
			'src-tauri/target/',
			'src-tauri/gen/',
			'dist/',
			'coverage/',
			'.claude/'
		]
	},
	{
		files: ['**/*.svelte'],
		rules: {
			// Disable custom element props warning - we're not building web components
			'svelte/valid-compile': ['error', { ignoreWarnings: true }]
		}
	}
);
