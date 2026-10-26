// Reuse the exact Step12 isolated fixture, launch, crop and persistence checks.
process.env.RESEARCHCLIP_LINKS_TEST='1';process.env.RESEARCHCLIP_EXCERPT_EXTENDED='1';
await import('./pdf-excerpts-native-smoke.mjs');
