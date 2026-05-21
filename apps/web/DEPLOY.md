# Deployment Guide — Genesis UI

This document outlines how to deploy the Genesis UI frontend to Vercel.

## 1. Automatic Deployment (GitHub)

The project is connected to the GitHub repository `Freshair129/cognitive_system`.

- **Branch watched**: `main`
- **Root Directory**: `apps/web`
- **Build Command**: `npm run build`
- **Output Directory**: `dist`

Every time you push to the `main` branch, Vercel will automatically trigger a production deployment.

## 2. Manual Deployment (Vercel CLI)

If you need to deploy manually or test a preview build before merging, use the Vercel CLI from the `apps/web/` directory.

### Preview Deployment

```bash
vercel
```

### Production Deployment

```bash
vercel --prod
```

## 3. Configuration Details

- **Vercel Project ID**: `prj_aELlyAqrhvnGBAPxl70HpE24DLrR`
- **Organization ID**: `team_ffvAb1lASX07dwqLeKwEAhlz`
- **Framework**: Vite
- **Node.js Version**: 20.x

## 4. Troubleshooting

### Build Failures

Ensure that `npm run build` passes locally before pushing. You can run this from the repo root:

```bash
npm run build --workspace=apps/web
```

### Environment Variables

If the UI requires API keys or special endpoints, configure them in the [Vercel Dashboard](https://vercel.com/freshair129/genesis-ui/settings/environment-variables).

---
*Created: 2026-05-16*
