import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';

import Upload from './Upload.svelte';

describe('Upload', () => {
  it('renders CSV upload guidance and direct Google import action', async () => {
    const onGoogleImport = vi.fn();

    render(Upload, {
      onUpload: vi.fn(),
      onGoogleImport,
    });

    expect(screen.getByRole('heading', { name: 'Upload Google Takeout CSV' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Google Takeout' })).toHaveAttribute(
      'href',
      'https://takeout.google.com'
    );
    expect(screen.getByRole('button', { name: /Drag & drop your CSV file here/ })).toBeInTheDocument();

    await fireEvent.click(screen.getByRole('button', { name: 'Import directly from Google Maps' }));

    expect(onGoogleImport).toHaveBeenCalledOnce();
  });
});
