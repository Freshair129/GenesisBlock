

interface Props {
  totalAtoms: number
  inboundCount: number
  hotfixCount: number
  candidatesCount?: number
}

export default function StatusBar({ totalAtoms, inboundCount, hotfixCount, candidatesCount }: Props) {
  return (
    <div className="status-bar">
      <span>{totalAtoms} atoms</span>
      <span>{hotfixCount} hotfixes</span>
      <span>{inboundCount} in inbound</span>
      {candidatesCount !== undefined && <span>{candidatesCount} candidates</span>}
    </div>
  )
}
