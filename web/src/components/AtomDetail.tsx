import { useEffect, useState } from 'react'
import { api } from '../api'

interface Props {
  atomId: string | null
}

export default function AtomDetail({ atomId }: Props) {
  const [atom, setAtom] = useState<any>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!atomId) {
      setAtom(null)
      setError(null)
      return
    }

    setLoading(true)
    api.getAtom(atomId)
      .then(data => {
        setAtom(data)
        setError(null)
      })
      .catch(err => {
        setError(err.message)
        setAtom(null)
      })
      .finally(() => {
        setLoading(false)
      })
  }, [atomId])

  if (!atomId) return <div className="detail-body">Select an atom</div>
  if (loading) return <div className="detail-body">Loading...</div>
  if (error) return <div className="detail-body">Error: {error}</div>
  if (!atom) return null

  // Extract frontmatter keys to display
  const fmKeys = Object.keys(atom).filter(k => k !== 'id' && k !== 'body')

  return (
    <>
      <div className="detail-header">
        <h3>{atom.id}</h3>
      </div>
      <div className="detail-body">
        <div style={{ marginBottom: '20px' }}>
          {fmKeys.map(k => (
            <div key={k}>
              <strong>{k}:</strong>{' '}
              {Array.isArray(atom[k]) 
                ? atom[k].map((v: string) => <span key={v} className="tag">{v}</span>)
                : atom[k]}
            </div>
          ))}
        </div>
        <hr style={{ borderColor: 'var(--border)', margin: '20px 0' }} />
        <div style={{ whiteSpace: 'pre-wrap' }}>
          {atom.body}
        </div>
      </div>
    </>
  )
}
