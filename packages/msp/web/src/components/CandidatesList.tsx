import { useEffect, useState } from 'react'
import { api, type CandidateRecord, type CandidateSummary } from '../api'

interface Props {
  onChanged?: () => void
}

export default function CandidatesList({ onChanged }: Props) {
  const [items, setItems] = useState<CandidateSummary[]>([])
  const [preview, setPreview] = useState<CandidateRecord | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)

  async function refresh() {
    try {
      setItems(await api.listCandidates())
    } catch (e: any) {
      setError(e.message)
    }
  }

  useEffect(() => {
    refresh()
  }, [])

  async function handleOpen(id: string) {
    setError(null)
    try {
      setPreview(await api.readCandidate(id))
    } catch (e: any) {
      setError(e.message)
    }
  }

  async function handleCopy(id: string) {
    setError(null)
    try {
      const record = await api.readCandidate(id)
      const fm = ['---']
      fm.push(`proposed_id: ${record.proposed_id}`)
      fm.push(`type: ${record.type}`)
      fm.push(`status: candidate`)
      fm.push(`proposed_at: ${record.proposed_at}`)
      fm.push(`proposed_by: ${record.proposed_by}`)
      if (record.rationale) fm.push(`rationale: ${record.rationale}`)
      if (record.confidence !== undefined) fm.push(`confidence: ${record.confidence}`)
      fm.push('---')
      const md = `${fm.join('\n')}\n\n${record.body}`.trim() + '\n'
      await navigator.clipboard.writeText(md)
    } catch (e: any) {
      setError(`Copy failed: ${e.message}`)
    }
  }

  async function handleDelete(id: string) {
    if (!confirm(`Delete candidate ${id}? This removes the file from .brain/.../candidates/.`)) return
    setBusyId(id)
    setError(null)
    try {
      await api.deleteCandidate(id)
      if (preview?.proposed_id === id) setPreview(null)
      await refresh()
      onChanged?.()
    } catch (e: any) {
      setError(e.message)
    } finally {
      setBusyId(null)
    }
  }

  return (
    <>
      <div className="atom-list-header">
        <strong>Candidates</strong>
        <span style={{ marginLeft: 8, opacity: 0.6, fontSize: '0.85em' }}>
          ({items.length})
        </span>
        <button onClick={refresh} style={{ marginLeft: 'auto' }} title="Refresh">
          ↻
        </button>
      </div>

      <div
        style={{
          padding: '6px 10px',
          fontSize: '0.78em',
          opacity: 0.75,
          borderBottom: '1px solid #e5e5e5',
        }}
      >
        Promote a candidate by copying its markdown into <code>gks/&lt;type&gt;/</code> and opening
        a PR. CI will validate.
      </div>

      {error && (
        <div style={{ padding: '6px 10px', color: '#b00020', fontSize: '0.85em' }}>{error}</div>
      )}

      <div className="atom-list">
        {items.length === 0 && (
          <div style={{ padding: 12, opacity: 0.6 }}>No candidates yet.</div>
        )}
        {items.map((c) => (
          <div key={c.proposed_id} className="atom-item">
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span
                style={{
                  fontSize: '0.7em',
                  padding: '2px 6px',
                  border: '1px solid #888',
                  borderRadius: 4,
                  textTransform: 'uppercase',
                }}
              >
                {c.type}
              </span>
              <span className="atom-id" style={{ flex: 1 }}>
                {c.proposed_id}
              </span>
            </div>
            <div className="atom-title" title={c.title}>
              {c.title || '(no title)'}
            </div>
            <div style={{ fontSize: '0.7em', opacity: 0.6 }}>
              {new Date(c.proposed_at).toLocaleString()} · {c.proposed_by}
            </div>
            <div style={{ display: 'flex', gap: 6, marginTop: 4 }}>
              <button onClick={() => handleOpen(c.proposed_id)}>Open</button>
              <button onClick={() => handleCopy(c.proposed_id)}>Copy markdown</button>
              <button
                onClick={() => handleDelete(c.proposed_id)}
                disabled={busyId === c.proposed_id}
              >
                Delete
              </button>
            </div>
          </div>
        ))}
      </div>

      {preview && (
        <div
          style={{
            position: 'fixed',
            top: '10%',
            left: '15%',
            right: '15%',
            bottom: '10%',
            background: 'white',
            border: '1px solid #888',
            borderRadius: 8,
            boxShadow: '0 4px 24px rgba(0,0,0,0.2)',
            padding: 16,
            overflow: 'auto',
            zIndex: 1000,
          }}
        >
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
            <h3 style={{ margin: 0 }}>{preview.proposed_id}</h3>
            <button onClick={() => setPreview(null)}>Close</button>
          </div>
          <div style={{ fontSize: '0.85em', opacity: 0.7, marginBottom: 8 }}>
            {preview.type} · {preview.proposed_by} · {new Date(preview.proposed_at).toLocaleString()}
            {preview.confidence !== undefined && ` · confidence ${preview.confidence}`}
          </div>
          {preview.rationale && (
            <div style={{ fontStyle: 'italic', marginBottom: 8 }}>{preview.rationale}</div>
          )}
          <pre
            style={{
              background: '#f7f7f7',
              padding: 12,
              borderRadius: 4,
              whiteSpace: 'pre-wrap',
              fontFamily: 'inherit',
              fontSize: '0.9em',
            }}
          >
            {preview.body}
          </pre>
        </div>
      )}
    </>
  )
}
