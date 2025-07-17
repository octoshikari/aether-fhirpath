// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://octoshikari.github.io',
	base: process.env.NODE_ENV === 'production' ? '/aether-fhirpath' : undefined,
	integrations: [
		starlight({
			title: 'Aether FHIRPath',
			description: 'A high-performance FHIRPath implementation in Rust with multiple language bindings.',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/octoshikari/aether-fhirpath' }
			],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'index' },
						{ label: 'Installation', slug: 'getting-started/installation' },
						{ label: 'Quick Start', slug: 'getting-started/quick-start' },
					],
				},
				{
					label: 'Usage',
					items: [
						{ label: 'CLI Tool', slug: 'usage/cli' },
						{ label: 'Rust Library', slug: 'usage/rust' },
						{ label: 'Node.js Bindings', slug: 'usage/nodejs' },
					],
				},
				{
					label: 'Examples',
					items: [
						{ label: 'Usage Examples', slug: 'examples/usage-examples' },
					],
				},
				{
					label: 'Development',
					items: [
						{ label: 'Contributing', slug: 'development/contributing' },
						{ label: 'Performance', slug: 'development/performance' },
						{ label: 'Implementation Plan', slug: 'development/implementation-plan' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Test Compliance', slug: 'reference/test-compliance' },
					],
				},
			],
		}),
	],
});
