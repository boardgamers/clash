// Acquisition instructions belong to the event, not to the card once it is in hand.
const greatPersonAcquisition =
  'You may take the Event Card to your hand for 1 culture token. If you pass, any other player (in player order) may take it for 2 culture tokens: Action card: ';

export function handCardDescription(description: string) {
  return description.startsWith(greatPersonAcquisition)
    ? description.slice(greatPersonAcquisition.length).trim()
    : description;
}
