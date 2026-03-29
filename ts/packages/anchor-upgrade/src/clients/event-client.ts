import { EventCodec } from '../codec/event-codec';
import type { DecodedEvent, EventByName, EventName, Idl } from '../types';

export class EventClient<IDL extends Idl, N extends EventName<IDL>> {
  private readonly codec: EventCodec<DecodedEvent<IDL, N>>;
  constructor(
    private readonly idl: IDL,
    private readonly eventDef: EventByName<IDL, N>,
  ) {
    this.codec = new EventCodec<DecodedEvent<IDL, N>>(this.idl, this.eventDef);
  }

  decode(base64Log: string): DecodedEvent<IDL, N> {
    return this.codec.decode(base64Log);
  }
}
