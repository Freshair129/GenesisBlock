

interface Props {
  totalAtoms: number
  hotfixCount: number
  candidatesCount?: number
}

export default function StatusBar({ totalAtoms, hotfixCount, candidatesCount }: Props) {
  return (
    <div className="status-bar">
      <span>{totalAtoms} atoms</span>
      <span>{hotfixCount} hotfixes</span>
      {candidatesCount !== undefined && <span>{candidatesCount} candidates</span>}
    </div>
  )
}
