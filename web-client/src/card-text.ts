// Acquisition instructions belong to the event, not to the card once it is in hand.
const greatPersonAcquisition =
  'You may take the Event Card to your hand for 1 culture token. If you pass, any other player (in player order) may take it for 2 culture tokens: Action card: ';

export function handCardDescription(description: string) {
  const prefixes = [greatPersonAcquisition,
    'You make place the Event Card in front of you 1 culture token. If you pass, any other player (in player order) may do so for 2 culture tokens: ',
  ];
  const prefix = prefixes.find((value) => description.startsWith(value));
  return prefix ? description.slice(prefix.length).trim() : description;
}
